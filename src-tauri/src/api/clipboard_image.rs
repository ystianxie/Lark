use arboard::{Clipboard, Error, ImageData};
use winapi::um::winuser::GetClipboardSequenceNumber;

pub(super) fn read_image_if_changed(
    clipboard: &mut Clipboard,
    last_sequence: &mut u32,
    sequence: u32,
) -> Result<ImageData<'static>, Error> {
    if sequence != 0 && sequence == *last_sequence {
        return Err(Error::ContentNotAvailable);
    }
    let image = clipboard.get_image();
    // A text/file read can succeed while the image read fails: track it separately.
    if (image.is_ok() || matches!(&image, Err(Error::ContentNotAvailable)))
        && unsafe { GetClipboardSequenceNumber() } == sequence
    {
        *last_sequence = sequence;
    }
    image
}
