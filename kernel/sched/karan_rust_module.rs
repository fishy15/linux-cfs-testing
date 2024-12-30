//! need this comment because its "documentation"

use kernel::prelude::*;

module! {
    type: KaranRustModule,
    name: "karan_rust_module",
    author: "karan gurazada",
    description: "hello",
    license: "MIT",
}

struct KaranRustModule {
    rust_counter: u64,
}

impl KaranRustModule {
    #[allow(dead_code)]
    fn karan_method () {
        pr_info!("hello! this is my method :3\n");
    }
}

impl kernel::karan::Karan for KaranRustModule {
    const USE_VTABLE_ATTR: () = ();
    
    #[allow(dead_code)]
    fn karan_trait_method () {
        pr_info!("hello! this is my implementation :3\n");
    }
}

impl kernel::Module for KaranRustModule {
    fn init(_module: &'static ThisModule) -> Result<Self> {
        pr_info!("karan_rust_module init\n");
        pr_info!("am i builtin? {}\n", !cfg!(MODULE));

        let rust_counter = 97;
        Ok(KaranRustModule{rust_counter})
    }
}

impl Drop for KaranRustModule {
    fn drop(&mut self) {
        pr_info!("my counter is {:?}\n", self.rust_counter);
        pr_info!("karan_rust_module exit\n");
    }
}


