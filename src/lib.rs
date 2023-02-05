#![feature(rustc_private)]
#![feature(let_chains)]

extern crate rustc_hir;
extern crate rustc_middle;

pub use rustc_codegen_llvm::ModuleLlvm;
use rustc_codegen_llvm::{
    context::CodegenCx,
    llvm::{
        LLVMDumpModule, LLVMGetFunctionAddress, LLVMLinkInMCJIT,
        LLVMRustCreateExecutionEngineForModule, LLVMRustGetNamedValue,
        LLVMRustLoadLibraryPermanently, LLVMSearchForAddressOfSymbol,
    },
};
pub use rustc_codegen_ssa::traits::CodegenBackend;
use rustc_codegen_ssa::{base::codegen_instance, traits::MiscMethods};
use rustc_hir::def_id::DefId;
use rustc_middle::{
    mir::visit::Visitor,
    ty::{Instance, InstanceDef, ParamEnv, TyCtxt},
};

use std::collections::HashSet;

pub fn llmod(tcx: TyCtxt, mod_name: &str) -> ModuleLlvm {
    ModuleLlvm::new(tcx, mod_name)
}

pub fn load_libstd() {
    let libdir = std::process::Command::new("rustc")
        .arg("--print=target-libdir")
        .output()
        .expect("Failed to run `rustc --print=sysroot`");
    let libdir = std::str::from_utf8(&libdir.stdout).unwrap().trim();

    for so in glob::glob(&format!("{}/libstd-*.so", libdir)).unwrap() {
        unsafe {
            LLVMRustLoadLibraryPermanently(
                so.unwrap().to_str().unwrap().as_ptr().cast()
            );
        }
    }
}

pub fn eval(tcx: TyCtxt, llmod: &ModuleLlvm, func: DefId) {
    let cx = CodegenCx::new(tcx, llmod);

    let mir = tcx.optimized_mir(func);

    let instance = Instance::mono(tcx, func);
    let instance_name = tcx.symbol_name(instance).name;
    cx.get_fn(instance);

    codegen_instance::<rustc_codegen_llvm::builder::Builder>(&cx, instance);

    unsafe {
        LLVMLinkInMCJIT();

        let mut ee = LLVMRustCreateExecutionEngineForModule(llmod.llmod());
        let addr =
            LLVMGetFunctionAddress(&ee, format!("{}\0", instance_name).as_str().as_ptr().cast());

        let f: extern "C" fn() -> () = std::mem::transmute(addr);
        f();
    }
}
