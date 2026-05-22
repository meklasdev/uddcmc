# Vendored `ilhook` — Page-Protection Fix

DarkClient ships a **patched, vendored copy of [`ilhook`](https://crates.io/crates/ilhook) `2.3.0`** under
[`vendor/ilhook`](../vendor/ilhook). Upstream `ilhook` 2.3.0 contains a Linux bug that
causes an **intermittent native crash whenever the client is injected**. This document
explains the bug, the evidence, and the fix.

---

## Symptom

Injecting into Minecraft would *sometimes* — not always — crash the JVM immediately
after the frame hook was installed. The client log ended normally:

```
[INFO] >>> HOOK ACTIVE ON: .../libglfw.so <<<
[INFO] DarkClient started
[INFO] GLFW window acquired; installing input callbacks.
```

…and the JVM died a few seconds later with a fatal error report (`hs_err_pid*.log`):

```
#  SIGSEGV (0xb) at pc=0x00007f7a60055ffd, pid=..., tid=...
#  Problematic frame:
#  C  0x00007f7a60055ffd
# The crash happened outside the Java Virtual Machine in native code.
```

The crash is **non-deterministic**: the exact same build, injected repeatedly, crashed
roughly **one time in four** and ran fine the rest.

---

## Background — how the frame hook works

DarkClient renders its overlay and drives module ticks by hooking the host's
buffer-swap function (`glfwSwapBuffers`). The hook is installed with `ilhook`, an
inline-hooking crate:

1. `ilhook` overwrites the first bytes of `glfwSwapBuffers` with a `jmp` to a
   **trampoline** it generates.
2. The trampoline saves registers, calls back into `swap_buffers_hook` (in
   `libclient.so`), restores registers, runs the relocated original instructions, and
   `jmp`s back into `glfwSwapBuffers`.

The trampoline is freshly generated machine code, so the memory holding it **must be
executable**.

---

## Root cause

The bug is in how `ilhook` 2.3.0 makes that trampoline memory executable on Unix.

### 1. The trampoline lives on the heap

`ilhook::x64::generate_trampoline` allocates the trampoline as an ordinary heap
allocation:

```rust
const TRAMPOLINE_MAX_LEN: usize = 1024;

fn generate_trampoline(...) -> Result<Box<[u8; TRAMPOLINE_MAX_LEN]>, HookError> {
    let mut trampoline_buffer = Box::new([0u8; TRAMPOLINE_MAX_LEN]);
    // ... machine code written into the box ...
}
```

It is a `Box<[u8; 1024]>` — **1024 bytes from the global allocator (`malloc`)**. Its
address is whatever the allocator hands back; it is only 16-byte aligned, **not**
page-aligned.

### 2. Only one page is made executable

After generating the trampoline, `ilhook` calls `modify_mem_protect` to mark it
`PROT_READ | PROT_WRITE | PROT_EXEC`. The upstream Unix implementation:

```rust
#[cfg(unix)]
fn modify_mem_protect(addr: usize, len: usize) -> Result<u32, HookError> {
    let page_size = unsafe { sysconf(30) }; // _SC_PAGESIZE
    if len > page_size.try_into().unwrap() {
        Err(HookError::InvalidParameter)
    } else {
        let ret = unsafe {
            mprotect(
                (addr & !(page_size as usize - 1)) as *mut c_void, // round start DOWN
                page_size as usize,                                // exactly ONE page
                7,                                                 // RWX
            )
        };
        // ...
    }
}
```

It rounds the start address **down** to a page boundary and `mprotect`s **exactly one
4 KiB page**. It implicitly assumes the whole `[addr, addr + len)` region fits inside
that single page.

### 3. A 1024-byte heap block frequently straddles a page boundary

That assumption is false. The trampoline is 1024 bytes placed at an arbitrary heap
address. If `malloc` returns an address in the **last 1024 bytes of a page**, the
trampoline spills across the page boundary into the **next** page:

```
        page N (rounded-down start)        page N+1
 ┌─────────────────────────────────┐┌──────────────────────────────┐
 ...       [ trampoline bytes .... ][ trampoline tail .. ]
           ^addr                    ^page boundary
           └── mprotect covers only page N ──┘
                                    └── tail stays NON-exec ──┘
```

`mprotect` makes page N executable. The trampoline tail that landed in page N+1 is
**never made executable** — it keeps the heap's default `rw-` protection.

### 4. Crash

When the render thread runs the hook, execution flows through the trampoline. The
moment the instruction pointer crosses into page N+1, the CPU fetches an instruction
from a **non-executable** page → `SIGSEGV` with `si_code = SEGV_ACCERR`.

### Why it is intermittent

The crash depends entirely on **where `malloc` placed the 1024-byte `Box`** within its
page, which varies run to run with heap layout and ASLR:

- Trampoline lands fully inside one page → hook works.
- Trampoline straddles a page boundary → tail is non-executable → crash.

The straddle probability is roughly `TRAMPOLINE_MAX_LEN / page_size = 1024 / 4096 ≈ 25%`
— matching the observed "≈ 1 in 4 injects" failure rate.

---

## Evidence from the crash report

The `hs_err_pid*.log` confirms every step.

**Faulting instruction crosses a page boundary.** The PC is `0x7f7a60055ffd`; the
instruction there is `f7 44 24 08 01 00 00 00` (`test dword [rsp+8], 1`), 8 bytes long,
so it spans `0x7f7a60055ffd … 0x7f7a60056004` — across the `0x7f7a60056000` boundary:

```
siginfo: si_signo: 11 (SIGSEGV), si_code: 2 (SEGV_ACCERR), si_addr: 0x00007f7a60056000
```

`SEGV_ACCERR` = the page exists but the access (instruction fetch) is not permitted.

**The callback target is `libclient.so`.** A register snapshot shows the trampoline was
about to `call` the DarkClient callback:

```
RAX=0x00007f7a4e785880: <offset 0x1585880> in /tmp/dark_client_..._libclient.so
```

The trampoline disassembly contains `mov rax, 0x7f7a4e785880; call rax` — the call into
`swap_buffers_hook`.

**The memory map shows exactly one executable page.** This is the smoking gun:

```
7f7a60055000-7f7a60056000 rwxp   ← the ONE page mprotect made executable
7f7a60056000-7f7a618e5000 rw-p   ← the next page: writable but NOT executable
```

The trampoline straddles `0x7f7a60056000`. Its head is in the `rwxp` page; its tail is
in the `rw-p` page. Execution crossing the boundary faults.

---

## The fix

The patched `ilhook` (in [`vendor/ilhook`](../vendor/ilhook)) changes `modify_mem_protect`
and `recover_mem_protect` to protect **every page the region touches**, not just the page
containing the start address — by rounding the start down *and* the end up:

```rust
#[cfg(unix)]
fn modify_mem_protect(addr: usize, len: usize) -> Result<u32, HookError> {
    if len == 0 {
        return Err(HookError::InvalidParameter);
    }
    let page_size = unsafe { sysconf(30) } as usize; // _SC_PAGESIZE
    // [addr, addr+len) may straddle a page boundary: the trampoline is a
    // heap-allocated `Box`, so its placement is arbitrary. Protect *every*
    // page the region touches, not just the one containing `addr`.
    let start = addr & !(page_size - 1);
    let end = (addr + len + page_size - 1) & !(page_size - 1);
    let ret = unsafe { mprotect(start as *mut c_void, end - start, 7) }; // RWX
    if ret != 0 {
        let err = unsafe { *(__errno_location()) };
        Err(HookError::MemoryProtect(err as u32))
    } else {
        Ok(7)
    }
}
```

`recover_mem_protect` gets the same start-down / end-up treatment (and now actually
uses its `len` argument, which upstream ignored).

The change is applied to **both** `src/x64.rs` and `src/x86.rs` for consistency,
although DarkClient only uses the x64 hooker.

### Suppressing the x86 ABI warnings

`ilhook`'s `x86` module emits `extern "cdecl"` ABI warnings when compiled on an x86-64
target. DarkClient only needs the x64 hooker, so `client/Cargo.toml` disables the
unused `x86` feature:

```toml
ilhook = { version = "2.3.0", default-features = false, features = ["x64"] }
```

The `x86` module is then not compiled at all — no warnings.

### How the vendored crate is wired in

The workspace root `Cargo.toml` redirects the crates.io `ilhook` dependency to the
local patched copy:

```toml
[patch.crates-io]
ilhook = { path = "vendor/ilhook" }
```

No call site changed — `client/src/graphic/hook.rs` still uses `ilhook::x64` exactly as
before. Only the dependency source is swapped.

---

## Maintenance notes

- **Do not delete `vendor/ilhook`** or remove the `[patch.crates-io]` entry — the crash
  returns immediately if the unpatched crates.io version is used.
- `ilhook` 2.3.0 is the latest published version; there is no upstream release to
  upgrade to. If a future version fixes this upstream, the vendored copy and the
  `[patch]` entry can be dropped — verify the fix is present in
  `modify_mem_protect`/`recover_mem_protect` first.
- The patch is intentionally minimal (page-span rounding only). The diff against
  upstream is limited to the two `#[cfg(unix)]` `*_mem_protect` functions in
  `src/x64.rs` and `src/x86.rs`.
