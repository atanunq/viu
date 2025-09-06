pub fn register_image_formats() {
    #[cfg(feature = "jpegxl")]
    register_jpegxl()
}

#[cfg(feature = "jpegxl")]
fn register_jpegxl() {
    const EXTENSION: &str = "jxl";

    use image::hooks::{register_format_detection_hook, register_decoding_hook};
    use jxl_oxide::integration::JxlDecoder;

    let success = register_decoding_hook(
        EXTENSION.into(),
        Box::new(|reader| Ok(Box::new(JxlDecoder::new(reader)?))),
    );
    if !success {
        eprintln!("Could not register jxl decoder.");
    }

    // SEE: https://docs.rs/image/0.25.8/image/hooks/fn.register_format_detection_hook.html#multiple-signatures
    let () = register_format_detection_hook(
        EXTENSION.into(),
        &[0xff, 0x0a],
        None,
    );
    let () = register_format_detection_hook(
        EXTENSION.into(),
         &[0x00, 0x00, 0x00, 0x0c, 0x4a, 0x58, 0x4c, 0x20, 0x0d, 0x0a, 0x87, 0x0a],
         None,
    );
}
