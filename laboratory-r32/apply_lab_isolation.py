from pathlib import Path
import json, hashlib
ROOT=Path.cwd()
changed={}
def edit(rel, old, new, count=1):
 p=ROOT/rel;s=p.read_text(encoding='utf-8')
 assert s.count(old)>=count, f"Missing baseline anchor: {rel}: {old[:70]}"
 p.write_text(s.replace(old,new,count),encoding='utf-8')
 changed[rel]=hashlib.sha256(p.read_bytes()).hexdigest()
cfg='libs/hbb_common/src/config.rs'
identity='BRS-Laboratorio-R32'
key='nW5Q3V4QqUANk3L6HIGU7Cg+YFWJ0iFRz2kh3oxJfAY='
edit(cfg,'RwLock::new("BRS".to_owned())',f'RwLock::new("{identity}".to_owned())')
edit(cfg,'&["rs-ny.rustdesk.com"]','&["127.0.0.1:22116"]')
edit(cfg,'pub fn get_rendezvous_server() -> String {','pub fn get_rendezvous_server() -> String {\n        return "127.0.0.1:22116".to_owned();')
edit(cfg,'pub fn get_rendezvous_servers() -> Vec<String> {','pub fn get_rendezvous_servers() -> Vec<String> {\n        return vec!["127.0.0.1:22116".to_owned()];')
options={'custom-rendezvous-server':'127.0.0.1:22116','relay-server':'127.0.0.1:22117','key':key,'api-server':'','approve-mode':'click','force-always-relay':'Y','direct-server':'N','enable-lan-discovery':'N','enable-remote-printer':'N','enable-file-transfer':'N','enable-tunnel':'N','enable-terminal':'N','disable-udp':'Y','allow-websocket':'N','allow-remote-config-modification':'N','enable-check-update':'N'}
match='\n        match k {\n'+''.join(f'            {json.dumps(k)} => return {json.dumps(v)}.to_owned(),\n' for k,v in options.items())+'            _ => {}\n        }\n'
edit(cfg,'pub fn get_option(k: &str) -> String {','pub fn get_option(k: &str) -> String {'+match)
for name in ['is_disable_installation','is_disable_account','is_disable_ab','is_disable_settings','is_disable_tcp_listen']:
 edit(cfg,f'pub fn {name}() -> bool {{',f'pub fn {name}() -> bool {{\n    return true;')
common='src/common.rs'
edit(common,'pub fn read_custom_client(config: &str) {','pub fn read_custom_client(config: &str) {\n    return; // Lab identity cannot be replaced by custom-client data.')
edit(common,'pub fn check_software_update() {','pub fn check_software_update() {\n    return; // Lab builds have no updater.')
edit(common,'pub fn get_custom_rendezvous_server(custom: String) -> String {','pub fn get_custom_rendezvous_server(custom: String) -> String {\n    return "127.0.0.1:22116".to_owned();')
edit(common,'pub fn get_api_server(api: String, custom: String) -> String {','pub fn get_api_server(api: String, custom: String) -> String {\n    return String::new();')
core='src/core_main.rs'
guard=r'''
    // Lab portable only: reject installation, service, elevation and URI entrypoints.
    let lab_args: Vec<String> = std::env::args().skip(1).collect();
    if let Some(first) = lab_args.first() {
        if !["--cm", "--cm-no-ui", "--server", "--tray", "--noinstall",
             "--version", "--build-date", "--get-id", "--connect",
             "--check-hwcodec-config", "multi_window"].contains(&first.as_str()) {
            return None;
        }
    }
'''
edit(core,'pub fn core_main() -> Option<Vec<String>> {','pub fn core_main() -> Option<Vec<String>> {'+guard)
# No automatic elevated portable service or cleanup of shared update staging.
edit(core,'crate::portable_service::client::set_quick_support(_is_quick_support);','_is_quick_support = false;\n        crate::portable_service::client::set_quick_support(false);')
edit(core,'crate::platform::try_remove_temp_update_files();','// Lab: do not touch shared update staging.')
# Refuse direct addresses or custom server syntax at outgoing connection boundary.
edit('src/client.rs','debug_assert!(peer == interface.get_id());','if peer.is_empty() || !peer.bytes().all(|c| c.is_ascii_digit()) {\n            bail!("Laboratorio: utiliza solamente un ID del laboratorio");\n        }\n        debug_assert!(peer == interface.get_id());')
for rel in ['flutter/windows/runner/main.cpp','flutter/windows/runner/win32_window.cpp']:
 p=ROOT/rel;s=p.read_text(encoding='utf-8')
 s=s.replace('Burzam.BRS','Burzam.BRS.Laboratorio.R32').replace('L"BRS"','L"BRS-Laboratorio-R32"').replace('FLUTTER_RUNNER_WIN32_WINDOW','BRS_LAB_R32_FLUTTER_WINDOW')
 p.write_text(s,encoding='utf-8');changed[rel]=hashlib.sha256(p.read_bytes()).hexdigest()
# Isolate every remaining literal branding profile bridge filename.
for rel in ['src/client.rs']:
 p=ROOT/rel;s=p.read_text().replace('BRS-technician-avatar.txt','BRS-Laboratorio-R32-technician-avatar.txt');p.write_text(s);changed[rel]=hashlib.sha256(p.read_bytes()).hexdigest()
report={'revision':'R32','lease_mode':'simulation','restrictions_enabled':False,'identity':identity,'rendezvous':'127.0.0.1:22116','relay':'127.0.0.1:22117','private_transport_required':True,'compiled':False,'files':changed}
Path('LAB_ISOLATION.json').write_text(json.dumps(report,indent=2))
print('LAB_ISOLATION_PATCH_APPLIED',len(changed))
