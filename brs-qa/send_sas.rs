pub fn send_sas() {
    #[link(name = "sas")]
    extern "system" {
        pub fn SendSAS(AsUser: BOOL);
    }
    // Serialize temporary changes made by this process.
    static SAS_POLICY_LOCK: Mutex<()> = Mutex::new(());
    let _guard = match SAS_POLICY_LOCK.lock() {
        Ok(guard) => guard,
        Err(e) => {
            log::error!("Cannot lock SAS policy: {}", e);
            return;
        }
    };
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let policy_key = match hklm.open_subkey_with_flags(
        "Software\\Microsoft\\Windows\\CurrentVersion\\Policies\\System",
        KEY_READ | KEY_WRITE,
    ) {
        Ok(key) => key,
        Err(e) => {
            log::error!("Cannot open SAS policy: {}", e);
            return;
        }
    };
    // None means genuinely absent, never a failed read or the DWORD value 0.
    let original = match policy_key.get_value::<u32, _>("SoftwareSASGeneration") {
        Ok(value) => Some(value),
        Err(e) if e.raw_os_error() == Some(ERROR_FILE_NOT_FOUND as i32) => None,
        Err(e) => {
            log::error!("Cannot read SAS policy: {}", e);
            return;
        }
    };
    let changed = !matches!(original, Some(1) | Some(3));
    if changed {
        if let Err(e) = policy_key.set_value("SoftwareSASGeneration", &1u32) {
            log::error!("Cannot prepare SAS policy: {}", e);
            return;
        }
    }
    log::info!("SAS received");
    unsafe { SendSAS(FALSE) };
    if changed {
        // Do not overwrite a different value installed by another writer.
        // Registry operations are not a cross-process transaction.
        match policy_key.get_value::<u32, _>("SoftwareSASGeneration") {
            Ok(1) => {}
            Ok(_) => {
                log::error!("SAS policy changed externally; restoration skipped");
                return;
            }
            Err(e) => {
                log::error!("Cannot verify SAS policy before restoration: {}", e);
                return;
            }
        }
        let restored = match original {
            Some(value) => policy_key.set_value("SoftwareSASGeneration", &value),
            None => policy_key.delete_value("SoftwareSASGeneration"),
        };
        if let Err(e) = restored {
            log::error!("Failed to restore SAS policy: {}", e);
        }
    }
}
