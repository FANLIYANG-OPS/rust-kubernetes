#![warn(unused_import_braces, unused_extern_crates)]
#![allow(missing_docs)]

pub(crate) mod vm;

use crate::{ImageTag, OsName, Vm, VmId, VmKind};
use core_def::VmId;
use myutil::{err::*, *};
pub fn pause(_id: VmId) -> Result<()> {
    Ok(())
}
pub fn resume(_vm: &Vm) -> Result<()> {
    Ok(())
}
