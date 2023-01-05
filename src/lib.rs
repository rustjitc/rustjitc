#![feature(rustc_private)]
#![feature(let_chains)]

extern crate rustc_hir;
extern crate rustc_middle;

pub use rustc_codegen_llvm::ModuleLlvm;
use rustc_codegen_llvm::{context::CodegenCx, llvm::LLVMRustGetNamedValue, llvm::{LLVMRustLoadLibraryPermanently, LLVMLinkInMCJIT, LLVMRustCreateExecutionEngineForModule, LLVMGetFunctionAddress, LLVMDumpModule}, llvm::LLVMSearchForAddressOfSymbol};
use rustc_codegen_ssa::{base::codegen_instance, traits::MiscMethods};
pub use rustc_codegen_ssa::traits::CodegenBackend;
use rustc_hir::def_id::DefId;
use rustc_middle::{
    mir::visit::Visitor,
    ty::{Instance, TyCtxt, ParamEnv, InstanceDef},
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
            dbg!(&so);
            dbg!(LLVMRustLoadLibraryPermanently(so.unwrap().to_str().unwrap().as_ptr().cast()));
        }
    }
}

// A visitor to collect pairs of dependent functions and its arguments
// and codegen those instances.

struct CallVisitor<'cx, 'll, 'tcx> {
    cx: &'cx CodegenCx<'ll, 'tcx>,
    instances: HashSet<Instance<'tcx>>,
}

impl<'cx, 'll, 'tcx> CallVisitor<'cx, 'll, 'tcx> {
    pub fn new(cx: &'cx CodegenCx<'ll, 'tcx>) -> Self {
        CallVisitor {
            cx,
            instances: HashSet::new(),
        }
    }
}

impl<'tcx> Visitor<'tcx> for CallVisitor<'_, '_, 'tcx> {
    fn visit_terminator(
        &mut self,
        terminator: &rustc_middle::mir::Terminator<'tcx>,
        location: rustc_middle::mir::Location,
    ) {
        match &terminator.kind {
            rustc_middle::mir::TerminatorKind::Call {
                func,
                args,
                destination,
                target,
                cleanup,
                from_hir_call,
                fn_span,
            } => {
                println!("-----------------");
                dbg!(terminator);
                dbg!(func, args);
                if let Some((func_did, substs)) = dbg!(func.const_fn_def())
                && let instance = dbg!(Instance::resolve(self.cx.tcx, ParamEnv::reveal_all(), func_did, substs).unwrap().unwrap().polymorphize(self.cx.tcx))
                && let mangled_instance_name = self.cx.tcx.symbol_name(instance).name
                && let addr = unsafe {dbg!(LLVMSearchForAddressOfSymbol(format!("{}\0", mangled_instance_name).as_str().as_ptr().cast()))}
                && addr.is_null() {
                    dbg!(self.cx.monomorphize(func.constant().unwrap()));
                    dbg!(instance.subst_mir_and_normalize_erasing_regions(self.cx.tcx, ParamEnv::reveal_all(), func.constant().unwrap().ty()));
                    dbg!(instance.ty(self.cx.tcx, ParamEnv::reveal_all()));
                    if let InstanceDef::Intrinsic(_) | InstanceDef::Virtual(_, _) = instance.def {
                        return;
                    }
                    self.cx.get_fn(instance);
                    codegen_instance::<rustc_codegen_llvm::builder::Builder>(self.cx, instance);
                    self.visit_body(self.cx.tcx.optimized_mir(instance.def_id()));
                }
                
                // if let Some((func, substs)) = func.const_fn_def() {
                //     let instance = Instance::resolve(self.cx.tcx, ParamEnv::reveal_all(), func, substs).unwrap().unwrap().polymorphize(self.cx.tcx);
                //     dbg!(instance);
                //     // if instance.def_id().is_local() {
                //         if let InstanceDef::Intrinsic(_) = instance.def {
                //             return;
                //         }
                //     self.cx.get_fn(instance);
                //         codegen_instance::<rustc_codegen_llvm::builder::Builder>(self.cx, instance);
                //         self.visit_body(self.cx.tcx.optimized_mir(instance.def_id()));
                //     // }
                // }
                // dbg!(func.const_fn_def());
                // if let Some((func, substs)) = func.const_fn_def()
                // // && func.is_local()
                // // && let instance = Instance::new(func, substs)
                // && let instance = Instance::resolve(self.cx.tcx, ParamEnv::reveal_all(), func, substs).unwrap().unwrap().polymorphize(self.cx.tcx)
                // && let mangled_instance_name = self.cx.tcx.symbol_name(instance).name
                // // && self.cx.get_declared_value(mangled_instance_name).is_none() {
                // {
                //     self.cx.codegen_operand()
                //     if let InstanceDef::Intrinsic(_) = instance.def {
                //         return;
                //     }
                //     dbg!(instance);
                //     // dbg!(self.cx.get_declared_value("_ZN3std4time7Instant3now17heaeb6a44de8a25dbE"));
                //     unsafe {
                //         // dbg!(LLVMRustGetNamedValue(self.cx.llmod, "_ZN3std4time7Instant3now17heaeb6a44de8a25dbE\0".as_ptr().cast(), "_ZN3std4time7Instant3now17heaeb6a44de8a25dbE".len()));
                //         // dbg!(LLVMSearchForAddressOfSymbol("_ZN3std4time7Instant3now17heaeb6a44de8a25dbE".as_ptr().cast()));
                //     }
                //     if unsafe {
                //         dbg!(LLVMSearchForAddressOfSymbol(format!("{}\0", mangled_instance_name).as_str().as_ptr().cast()))
                //     }.is_null() {
                //         if dbg!(self.cx.get_declared_value(mangled_instance_name)).is_none() {
                //             self.instances.insert(Instance::new(func, substs));
                //             self.visit_body(self.cx.tcx.optimized_mir(func));
                //         }    
                //     }
                // }
            }
            _ => {}
        }
    }
}

pub fn eval(tcx: TyCtxt, llmod: &ModuleLlvm, func: DefId) {
    let cx = CodegenCx::new(tcx, llmod);

    let mir = tcx.optimized_mir(func);

    let instance = Instance::mono(tcx, func);
    // cx.get_fn(instance);
    
    let mut visitor = CallVisitor::new(&cx);
    visitor.visit_body(mir);

    // let instances_to_codegen = visitor.instances;
    // dbg!(&instances_to_codegen);
    // for instance in instances_to_codegen {
        // codegen_instance::<rustc_codegen_llvm::builder::Builder>(&cx, instance);
    // }

    codegen_instance::<rustc_codegen_llvm::builder::Builder>(&cx, instance);
    let instance_name = tcx.symbol_name(instance).name;
    dbg!(instance_name);

    unsafe {
        // LLVMDumpModule(llmod.llmod());
        // dbg!(LLVMSearchForAddressOfSymbol("_ZN8rust_out4main17h0167b3d0037b9fe7E\0".as_ptr().cast()));
        LLVMLinkInMCJIT();
        
        let mut ee = LLVMRustCreateExecutionEngineForModule(llmod.llmod());
        let addr = LLVMGetFunctionAddress(&ee, format!("{}\0", instance_name).as_str().as_ptr().cast());
        
        let f: extern "C" fn(i32) -> (i32) = std::mem::transmute(addr);
        dbg!(f);
        dbg!(f(1));
    }
}

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
