use std::fs::File;
use std::io::{BufRead, BufReader};
use crate::client::DarkClient;
use crate::mapping::client::minecraft::Minecraft;
use cfg_if::cfg_if;
use log::{info, error};
use std::sync::atomic::{AtomicI32, AtomicBool, Ordering};
use std::sync::{Mutex, Once, OnceLock};
use std::time::Instant;
use ilhook::x64::HookPoint;
use libc::{RTLD_GLOBAL, RTLD_LAZY};
// Importa il crate gl per disegnare
use crate::{gl, RUNNING};

static LAST_TICK: AtomicI32 = AtomicI32::new(0);
static GL_LOADED: AtomicBool = AtomicBool::new(false);

// Creiamo un wrapper per aggirare il blocco del compilatore
pub struct HookHandle(HookPoint);

// DICIAMO A RUST: "Fidati, posso spostare questo oggetto tra thread"
unsafe impl Send for HookHandle {}
unsafe impl Sync for HookHandle {}

// Global storage per l'hook attivo.
// Usiamo un Mutex per poterlo modificare (rimuovere) a runtime.
// Nota: Il tipo esatto dipende da cosa restituisce hooker.hook().
// Solitamente è un oggetto che implementa Drop o ha un metodo unhook.
// Per ilhook-rs, l'oggetto `Hook` gestisce l'unhooking quando viene droppato.
// Aggiorniamo lo storage globale per usare questo tipo specifico, non dyn Any
static GLOBAL_HOOK: OnceLock<Mutex<Option<HookHandle>>> = OnceLock::new();

fn get_global_hook() -> &'static Mutex<Option<HookHandle>> {
    GLOBAL_HOOK.get_or_init(|| Mutex::new(None))
}

cfg_if! {
    if #[cfg(target_os = "linux")] {
        use std::ffi::{CString, CStr};
        use ilhook::x64::{Hooker, Registers, CallbackOption, HookFlags, HookType};
        use libc::{c_void, c_char};

        // Helper per caricare le funzioni OpenGL su Linux
        fn get_proc_address(addr: &str) -> *const c_void {
            unsafe {
                let s = CString::new(addr).unwrap();
                // Prova prima con glXGetProcAddress se disponibile, altrimenti dlsym
                // Qui usiamo un approccio semplificato assumendo che libGL sia caricata
                let lib = libc::dlopen(CString::new("libGL.so.1").unwrap().as_ptr(), libc::RTLD_LAZY);
                if !lib.is_null() {
                    libc::dlsym(lib, s.as_ptr())
                } else {
                    std::ptr::null()
                }
            }
        }

        unsafe extern "win64" fn my_swap_buffers_hook(_regs: *mut Registers, _user_data: usize) {
            on_frame();
        }

    } else if #[cfg(target_os = "windows")] {
        use ilhook::x64::{Hooker, Registers, CallbackOption, HookFlags, HookType};
        use libloading::os::windows::{Library, Symbol};
        use std::ffi::CString;

        // Helper per caricare le funzioni OpenGL su Windows
        fn get_proc_address(addr: &str) -> *const std::ffi::c_void {
            unsafe {
                // Metodo robusto: prova wglGetProcAddress, poi GetProcAddress
                let c_str = CString::new(addr).unwrap();

                // Nota: In un contesto reale, dovremmo aver caricato opengl32.dll staticamente o lazy
                // Qui facciamo un tentativo "sporco" ma funzionale per l'injection
                let lib = libloading::Library::new("opengl32.dll");
                if let Ok(l) = lib {
                     // Prima prova wglGetProcAddress (per estensioni moderne)
                     let wgl_get: Result<libloading::Symbol<unsafe extern "system" fn(*const i8) -> *const std::ffi::c_void>, _> = l.get(b"wglGetProcAddress");
                     if let Ok(wgl) = wgl_get {
                         let ptr = wgl(c_str.as_ptr());
                         if !ptr.is_null() {
                             return ptr;
                         }
                     }
                     // Fallback a GetProcAddress (per funzioni base GL 1.1)
                     let func: Result<libloading::Symbol<unsafe extern "system" fn()>, _> = l.get(c_str.as_bytes());
                     if let Ok(f) = func {
                         return *f as *const std::ffi::c_void;
                     }
                }
                std::ptr::null()
            }
        }

        unsafe extern "win64" fn my_swap_buffers_hook(_regs: *mut Registers, _user_data: usize) {
            on_frame();
        }
    }
}

const CONTEXT_PROFILE_MASK: u32 = 0x9126;
const CONTEXT_CORE_PROFILE_BIT: u32 = 0x00000001;

// Funzione chiamata ogni frame grafico
unsafe fn on_frame() {
    // === PANIC CHECK (La modifica importante) ===
    // Se cleanup_client() è stato chiamato, RUNNING diventa false.
    // Se è false, usciamo IMMEDIATAMENTE. Non tickiamo, non disegniamo.
    if !RUNNING.load(Ordering::SeqCst) {
        return;
    }

    // Inizializza GL pointers...
    if !GL_LOADED.load(Ordering::Relaxed) {
        gl::load_with(|s| get_proc_address(s));
        GL_LOADED.store(true, Ordering::Relaxed);
    }

    check_tick();
    render_overlay();
}

// === LOGICA DI RENDERING (Fix per i glitch grafici) ===
unsafe fn render_overlay() {
    // === 1. DATI FINESTRA ===
    // Otteniamo le dimensioni reali della finestra (funziona anche se ridimensioni)
    let mut viewport = [0; 4]; // [x, y, width, height]
    gl::GetIntegerv(gl::VIEWPORT, viewport.as_mut_ptr());

    let screen_w = viewport[2];
    let screen_h = viewport[3];

    // === 2. BACKUP STATO ===
    let is_scissor_on = gl::IsEnabled(gl::SCISSOR_TEST) == gl::TRUE;
    let mut old_scissor_box = [0; 4];
    gl::GetIntegerv(gl::SCISSOR_BOX, old_scissor_box.as_mut_ptr());
    let mut old_clear_color = [0.0; 4];
    gl::GetFloatv(gl::COLOR_CLEAR_VALUE, old_clear_color.as_mut_ptr());

    // Abilita il taglio
    gl::Enable(gl::SCISSOR_TEST);

    // === 3. DISEGNO PIXEL ART (Helper Closure) ===
    // Definiamo un colore Giallo
    gl::ClearColor(1.0, 1.0, 0.0, 1.0);

    // Funzione interna per disegnare un blocco
    // x, y sono relativi all'angolo IN ALTO A SINISTRA
    let draw_block = |x: i32, y: i32, w: i32, h: i32| {
        // OpenGL ha (0,0) in BASSO a sinistra. Dobbiamo invertire la Y.
        // Y_gl = AltezzaSchermo - Y_alto - AltezzaBlocco
        let gl_y = screen_h - y - h;

        gl::Scissor(x, gl_y, w, h);
        gl::Clear(gl::COLOR_BUFFER_BIT);
    };

    // --- DISEGNO LA "D" (Di Dark) ---
    // Posizione: 10px dal bordo sinistro, 10px dal bordo alto
    let start_x = 10;
    let start_y = 10;
    let thickness = 5;
    let size = 30; // Altezza lettera

    // Asta Verticale
    draw_block(start_x, start_y, thickness, size);
    // Trattino Alto
    draw_block(start_x, start_y, size - 10, thickness);
    // Trattino Basso
    draw_block(start_x, start_y + size - thickness, size - 10, thickness);
    // Asta Destra (chiusura D)
    draw_block(start_x + size - 10, start_y + thickness, thickness, size - (thickness * 2));

    // --- DISEGNO LA "C" (Di Client) ---
    // Spostiamoci a destra
    let start_x = 50;

    // Asta Verticale
    draw_block(start_x, start_y, thickness, size);
    // Trattino Alto
    draw_block(start_x, start_y, size - 5, thickness);
    // Trattino Basso
    draw_block(start_x, start_y + size - thickness, size - 5, thickness);

    // === 4. RIPRISTINO STATO ===
    gl::ClearColor(old_clear_color[0], old_clear_color[1], old_clear_color[2], old_clear_color[3]);
    gl::Scissor(old_scissor_box[0], old_scissor_box[1], old_scissor_box[2], old_scissor_box[3]);
    if !is_scissor_on {
        gl::Disable(gl::SCISSOR_TEST);
    }
}

fn check_tick() {
    let client = DarkClient::instance();
    // Try to get env without attaching if possible, or attach as daemon.
    let _env = match client.jvm.attach_current_thread_as_daemon() {
        Ok(env) => env,
        Err(_) => return,
    };

    let minecraft = Minecraft::instance();

    // Controllo errori per evitare crash
    if minecraft.player.entity.is_null() {
        return;
    }

    let tick_count = match minecraft.player.entity.get_tick_count() {
        Ok(t) => t,
        Err(_) => return,
    };

    let last_tick = LAST_TICK.load(Ordering::Relaxed);

    if tick_count > last_tick {
        LAST_TICK.store(tick_count, Ordering::Relaxed);
        client.tick();
    }
}

fn find_library_path(partial_name: &str) -> Option<String> {
    if let Ok(file) = File::open("/proc/self/maps") {
        let reader = BufReader::new(file);
        for line in reader.lines() {
            if let Ok(l) = line {
                // Cerchiamo una riga che contenga il nome (es. "liblwjgl_opengl.so")
                if l.contains(partial_name) && l.contains(".so") {
                    // Il formato è: indirizzo permessi offset dev inode PERCORSO
                    // Prendiamo l'ultima parte della stringa
                    if let Some(path) = l.split_whitespace().last() {
                        return Some(path.to_string());
                    }
                }
            }
        }
    }
    None
}

pub fn install_hooks() -> anyhow::Result<()> {
    cfg_if! {
        if #[cfg(target_os = "linux")] {
            unsafe {
                let mut targets = Vec::new();

                if let Some(path) = find_library_path("libglfw.so") {
                    info!("Trovata libreria GLFW: {}", path);
                    targets.push((path, "glfwSwapBuffers"));
                }
                else if let Some(path) = find_library_path("liblwjgl.so") {
                    info!("Trovata libreria LWJGL (Legacy): {}", path);
                    targets.push((path, "glXSwapBuffers"));
                }
                else {
                    info!("Nessuna libreria specifica trovata, provo libGL di sistema...");
                    targets.push(("libGL.so.1".to_string(), "glXSwapBuffers"));
                }

                let mut hooked_count = 0;

                for (lib_path, func_name) in targets {
                    let c_lib_path = CString::new(lib_path.clone())?;
                    let c_func_name = CString::new(func_name)?;

                    let lib = libc::dlopen(c_lib_path.as_ptr(), libc::RTLD_LAZY);

                    if !lib.is_null() {
                        let target_addr = libc::dlsym(lib, c_func_name.as_ptr()) as usize;

                        if target_addr != 0 {
                            info!("Trovato {} in {} a 0x{:x}", func_name, lib_path, target_addr);

                            let hooker = Hooker::new(
                                target_addr,
                                HookType::JmpBack(my_swap_buffers_hook),
                                CallbackOption::None,
                                0,
                                HookFlags::empty()
                            );

                            match hooker.hook() {
                                Ok(hook) => {
                                    // Recuperiamo il lock globale
                                    let mut guard = get_global_hook().lock().unwrap();

                                    // Se c'era un hook vecchio, lo sovrascriviamo (triggerando l'unhook automatico del vecchio)
                                    if guard.is_some() {
                                        info!("Rilevato vecchio hook, rimozione e sostituzione...");
                                    }

                                    // === QUI LA MAGIA ===
                                    // Avvolgiamo l'hook nel nostro wrapper "Thread Safe"
                                    *guard = Some(HookHandle(hook));

                                    hooked_count += 1;
                                    info!(">>> HOOK ATTIVO E SALVATO SU: {} <<<", lib_path);

                                    // Usciamo dal loop, ne basta uno attivo
                                    break;
                                },
                                Err(e) => {
                                    info!("Errore installazione hook su {}: {:?}", lib_path, e);
                                }
                            }
                        }
                    }
                }

                if hooked_count == 0 {
                    // Qui potremmo ritornare errore, MA se stiamo re-iniettando e qualcosa è andato storto
                    // col flag statico, potremmo voler "fingere" che vada tutto bene.
                    // Tuttavia, per ora lasciamo l'errore se count è 0.
                    return Err(anyhow::anyhow!("Fallito l'hook su tutte le librerie candidate!"));
                }
            }
        } else if #[cfg(target_os = "windows")] {
             // ... Codice Windows (non cambia molto, ma aggiungi il check HOOKS_INSTALLED se vuoi) ...
             // Per brevità ometto, ma il concetto è identico.
        }
    }
    Ok(())
}

pub fn uninstall_hooks() {
    // Prendiamo il lock
    let mut guard = get_global_hook().lock().unwrap();

    if guard.is_some() {
        info!("Rimozione hook fisico in corso...");
        // Impostando a None, il Wrapper viene distrutto.
        // Il Wrapper distrugge l'HookPoint interno.
        // L'HookPoint interno ripristina i byte originali della memoria.
        *guard = None;
        info!("Hook rimosso correttamente. Memoria pulita.");
    } else {
        info!("Nessun hook attivo da rimuovere.");
    }
}