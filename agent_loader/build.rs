fn main() {
    // On all platforms, agent_loader resolved its only JNI dependency
    // (JNI_GetCreatedJavaVMs) dynamically at runtime, so we do not
    // need to search for or link against jvm.lib at build time.
}
