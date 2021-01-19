mod def;
#[cfg(feature = "testmock")]
mod mocker;

pub use mocker::*;
pub use def::*;