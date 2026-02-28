use tauri_winres;

// static WINDOWS_MANIFEST_RESOURCE: &'static str = r#"
// <?xml version="1.0" encoding="UTF-8" standalone="yes"?>
// <assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
// 	<trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
// 		<security>
// 			<requestedPrivileges>
// 				<requestedExecutionLevel level="requireAdministrator" uiAccess="false"/>
// 			</requestedPrivileges>
// 		</security>
// 	</trustInfo>
// </assembly>
// "#;

// Configures windows application resource.( fix for app icon and launching app as admin)
#[cfg(windows)]
fn main() {
    let mut res = tauri_winres::WindowsResource::new();
    res.set_icon("static/appIcons/icon.ico");
    // res.set_manifest(WINDOWS_MANIFEST_RESOURCE);
    res.compile().unwrap();
}

#[cfg(unix)]
fn main() {}
