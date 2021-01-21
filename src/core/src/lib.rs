#![cfg(target_os = "linux")]
#![warn(missing_docs, unused_import_braces, unused_extern_crates)]

mod def;
pub use def::*;

mod mocker;
pub use mocker::*;

mod common {
    use crate::Vm;
    use std::path::PathBuf;
    pub(crate) const CLONE_MARK: &str = "_clone";

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
