#![cfg(target_os = "linux")]
#![warn(missing_docs, unused_import_braces, unused_extern_crates)]

mod def;
pub use def::*;

mod mocker;
pub use mocker::*;

mod linux;
pub use linux::*;

mod common {

    use crate::Vm;
    use std::path::PathBuf;
    pub(crate) const CLONE_MARK: &str = "clone_";
    use futures::executor::{ThreadPool, ThreadPoolBuilder};
    use lazy_static::lazy_static;
    use myutil::{err::*, *};

    lazy_static! {
        pub(crate) static ref THREAD_POOL: ThreadPool =
            pnk!(ThreadPoolBuilder::new().pool_size(5).create());
        pub(crate) static ref ZFS_ROOT: &'static str = pnk!(img_root_register(None));
    }

    pub(crate) async fn async_sleep(t: u64) {
        futures_timer::Delay::new(std::time::Duration::from_secs(t)).await;
    }

    pub fn img_root_register(img_path: Option<&str>) -> Option<&'static str> {
        static mut ROOT: Option<String> = None;
        if let Some(path) = img_path {
            unsafe {
                ROOT.replace(
                    path.trim_start_matches("/dev/zvol")
                        .trim_end_matches("/")
                        .to_owned(),
                );
            }
        }
        unsafe { ROOT.as_deref() }
    }

    #[inline(always)]
    pub fn vm_img_path(vm: &Vm) -> PathBuf {
        let mut vm_img_path = vm.image_path.clone();
        let vm_img_name = format!("{}{}", CLONE_MARK, vm.id);
        vm_img_path.set_file_name(vm_img_name);
        vm_img_path
    }
}

pub(crate) use common::*;

mod test;
