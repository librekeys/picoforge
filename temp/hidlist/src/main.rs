use std::io::Write;
fn main() {
    let mut out = String::new();
    let api = hidapi::HidApi::new().unwrap();
    let mut fido = 0;
    for d in api.device_list() {
        if d.usage_page() == 0xF1D0 {
            fido += 1;
            out.push_str(&format!("hidapi FIDO entry: vid=0x{:04X} pid=0x{:04X} iface={} path={:?}\n",
                d.vendor_id(), d.product_id(), d.interface_number(), d.path()));
        }
    }
    out.push_str(&format!("hidapi FIDO entries={fido}\n"));
    let path = r"C:\Users\guoxin\Desktop\桌面文件夹\项目\picoforge\temp\hidlist\elev_test.txt";
    std::fs::File::create(path).unwrap().write_all(out.as_bytes()).unwrap();
    print!("{out}");
}
