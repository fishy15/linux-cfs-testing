//! some bullshit

use crate::prelude::*;

/// vtableing it up
#[vtable]
pub trait Karan {
    /// je suis un methode
    fn karan_trait_method () {
        pr_info!("hello! i am the default implementation\n");
    }
}

/// this function is flown
#[no_mangle]
pub extern "C" fn karan_extern_function () {
    pr_info!("please help\n");
}

/// this function is flown
#[no_mangle]
pub extern "C" fn karan_function () {
    pr_info!("please help\n");
}
