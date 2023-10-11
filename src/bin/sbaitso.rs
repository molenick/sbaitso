use tts::*;

slint::include_modules!();

#[cfg(target_os = "macos")]
use cocoa_foundation::base::id;
#[cfg(target_os = "macos")]
use cocoa_foundation::foundation::NSRunLoop;
#[cfg(target_os = "macos")]
use objc::{msg_send, sel, sel_impl};

fn main() -> Result<(), Error> {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut tts = Tts::default()?;

    // Provide NSLoop for tts on macOS
    #[cfg(target_os = "macos")]
    rt.spawn(async move {
        {
            let run_loop: id = unsafe { NSRunLoop::currentRunLoop() };
            unsafe {
                let _: () = msg_send![run_loop, run];
            }
        }
    });

    tts.speak("Initializing SBAITSO", false)?;
    let window = MainWindow::new().unwrap();
    window.run().unwrap();

    Ok(())
}
