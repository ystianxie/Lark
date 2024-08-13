#[cfg(target_os = "macos")]
pub fn get_clipboard_files() -> Vec<(String, String)> {
    use cocoa::appkit::NSPasteboard;
    use cocoa::base::nil;
    use cocoa::foundation::{NSArray, NSAutoreleasePool};
    use objc::{msg_send, sel, sel_impl};
    use objc::runtime::{Class, Object};
    unsafe {
        let _pool = NSAutoreleasePool::new(nil);
        let pasteboard = NSPasteboard::generalPasteboard(nil);

        let classes = vec![
            Class::get("NSURL").unwrap() as *const _ as *mut Object,
        ];

        let objects = pasteboard.readObjectsForClasses_options(
            NSArray::arrayWithObjects(nil, &classes),
            nil,
        );

        if objects.is_null() {
            return Vec::new();
        }

        let count = objects.count();
        let mut files = Vec::new();
        let home_dir = tauri::api::path::home_dir().unwrap().to_str().unwrap().to_string();
        for i in 0..count {
            let object = objects.objectAtIndex(i);
            let url: *mut Object = object as *mut Object;

            let path: *mut Object = msg_send![url, path];
            let extension: *mut Object = msg_send![url, pathExtension];

            if !path.is_null() {
                let path_str: *const libc::c_char = msg_send![path, UTF8String];
                let mut path_string = std::ffi::CStr::from_ptr(path_str).to_string_lossy().into_owned();
                let mut extension_string = if !extension.is_null() {
                    let extension_str: *const libc::c_char = msg_send![extension, UTF8String];
                    std::ffi::CStr::from_ptr(extension_str).to_string_lossy().into_owned()
                } else {
                    String::new()
                };
                if extension_string.is_empty() {
                    let path_ = std::path::Path::new(&path_string);
                    if path_.is_dir() {
                        extension_string = "folder".to_string();
                    }
                }
                path_string = path_string.replace(&home_dir, "~");
                files.push((path_string,extension_string));
            }
        }

        files
    }
}


#[cfg(target_os = "windows")]
pub fn get_clipboard_files() -> Vec<(String,String)> {
    use clipboard_win::raw;
    let mut files = Vec::new();
    let _ = raw::open();
    let _ = raw::get_file_list(&mut files);
    let _ = raw::close();
    let mut files_data = Vec::new();
    for file in files {
        let file_path = std::path::Path::new(&file);
        if file_path.is_dir(){
            files_data.push((file.clone(),"folder".to_string()));
        }else {
            let file_extension = file_path.extension().unwrap().to_string_lossy().to_string();
            files_data.push((file.clone(),file_extension.clone()));
        }
    }
    files_data
}
