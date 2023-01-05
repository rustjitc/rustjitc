#![feature(rustc_private)]

use std::path::PathBuf;

use rustc_interface::Config;
use rustc_session::config::Options;
use rustc_span::FileName;

extern crate rustc_codegen_ssa;
extern crate rustc_errors;
extern crate rustc_hash;
extern crate rustc_hir;
extern crate rustc_interface;
extern crate rustc_metadata;
extern crate rustc_middle;
extern crate rustc_session;
extern crate rustc_span;
extern crate rustc_driver;

fn rustc_config(sysroot: &str, code: String) -> Config {
    Config {
        opts: Options {
            maybe_sysroot: Some(PathBuf::from(sysroot)),
            ..Default::default()
        },
        crate_cfg: Default::default(),
        crate_check_cfg: Default::default(),
        input: rustc_session::config::Input::Str { name: FileName::Custom("IRS".to_owned()), input: code },
        input_path: None,
        output_file: None,
        output_dir: None,
        file_loader: None,
        lint_caps: Default::default(),
        parse_sess_created: None,
        register_lints: None,
        override_queries: None,
        make_codegen_backend: None,
        registry: rustc_driver::diagnostics_registry(),
    }
}

#[test]
fn foobar() {}