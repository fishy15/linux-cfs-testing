// SPDX-License-Identifier: GPL-2.0

use crate::helpers::*;
use proc_macro::{token_stream, TokenStream, TokenTree};

#[derive(Debug, Default)]
struct KaranInfo {
    type_: String,
    name: String,
}

impl KaranInfo {
    fn parse(it: &mut token_stream::IntoIter) -> Self {
        let mut info = KaranInfo::default();

        const EXPECTED_KEYS: &[&str] =
            &["type", "name"];
        const REQUIRED_KEYS: &[&str] = &["type", "name"];
        let mut seen_keys = Vec::new();

        loop {
            let key = match it.next() {
                Some(TokenTree::Ident(ident)) => ident.to_string(),
                Some(_) => panic!("Expected Ident or end"),
                None => break,
            };

            if seen_keys.contains(&key) {
                panic!(
                    "Duplicated key \"{}\". Keys can only be specified once.",
                    key
                );
            }

            assert_eq!(expect_punct(it), ':');

            match key.as_str() {
                "type" => info.type_ = expect_ident(it),
                "name" => info.name = expect_string_ascii(it),
                _ => panic!(
                    "Unknown key \"{}\". Valid keys are: {:?}.",
                    key, EXPECTED_KEYS
                ),
            }

            assert_eq!(expect_punct(it), ',');

            seen_keys.push(key);
        }

        expect_end(it);

        for key in REQUIRED_KEYS {
            if !seen_keys.iter().any(|e| e == key) {
                panic!("Missing required key \"{}\".", key);
            }
        }

        let mut ordered_keys: Vec<&str> = Vec::new();
        for key in EXPECTED_KEYS {
            if seen_keys.iter().any(|e| e == key) {
                ordered_keys.push(key);
            }
        }

        if seen_keys != ordered_keys {
            panic!(
                "Keys are not ordered as expected. Order them like: {:?}.",
                ordered_keys
            );
        }

        info
    }
}

// assuming that a module already exists
// gross!
pub(crate) fn karan_macro(ts: TokenStream) -> TokenStream {
    let mut it = ts.into_iter();
    let info = KaranInfo::parse(&mut it);

    format!(
        "
            /// extern
            #[no_mangle]
            pub extern \"C\" fn __{name}_karan() -> core::ffi::c_int {{
                __karan()
            }}

            /// dunder
            fn __karan() -> core::ffi::c_int {{
                pr_info!(\"what have i done??\n\");
                unsafe {{
                if let Some(mut MOD) = __MOD.as_mut() {{
                    match MOD.karan() {{
                        Ok(m) => 0,
                        Err(e) => e.to_errno(),
                    }}
                }} else {{ -1 }} }}
            }}
        ",
        name = info.name,
    )
    .parse()
    .expect("Error parsing formatted string into token stream.")
}
