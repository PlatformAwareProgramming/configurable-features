use proc_macro::TokenStream;
use configurable_internal::__internal_configurable;

/// Marks a module as a **Kernel Module**, enabling structured platform-aware programming.
///
/// This attribute transforms the annotated module into a dispatch system for platform-specific
/// function variants. It enables you to define multiple implementations of the same logic,
/// automatically selecting the most optimized version at runtime based on the available hardware.
///
/// # How it works
///
/// 1. **Variant Grouping**: Functions inside this module (whether standalone or in `impl` blocks)
///    are treated as **Kernel Functions** (or *k-functions*). The module itself acts as a **Kernel Module**,
///    encapsulating coexisting implementations that differ in their assumptions about the features
///    of the underlying computing platform.
///
/// 2. **Hardware Constraints**: Use the `#[assumptions(...)]` attribute to specify requirements.
///    The set of platform arguments defines the assumption for implementing the k-function version,
///    allowing you to target specific features like SIMD sets (`cpu_simd`) or Accelerator Models (`acc_model`).
///
/// 3. **Fallback Requirement**: You **must** provide a fallback. The first one declared is the
///    fallback version, which does not declare any assumptions because it must be executed on
///    any computing platform.
///
/// 4. **Dispatch Generation**: The macro generates a dispatcher that runs a **Resolution Algorithm**.
///    At the beginning of execution, each kernel module... selects a k-function whose assumption
///    is compatible with the features of the underlying computing platform, prioritizing the most
///    specific valid assumption ($P <: K_n < ... < K_0$).
///
/// # Examples
///
/// ## 1. Standalone Functions
/// Use `configurable` to create simple function variants (e.g., a fast SIMD version and a scalar fallback).
///
/// ```rust
/// #[configurable]
/// mod vector_math {
///
///     // Variant 2: Standard Scalar Fallback (Required)
///     #[assumptions]
///     fn add_vectors(a: &[i32], b: &[i32]) -> Vec<i32> {
///         a.iter().zip(b).map(|(x, y)| x + y).collect()
///     }
///
///     // Variant 1: Optimized for AVX2 instruction sets
///     #[assumptions(cpu_simd = AVX2)]
///     fn add_vectors(a: &[i32], b: &[i32]) -> Vec<i32> {
///         // ... AVX2 optimized implementation ...
///         vec![]
///     }
/// }
/// ```
///
/// ## 2. External File Inclusion
/// To keep your code modular, you can define your kernels in separate files (e.g., one file for
/// NVIDIA logic, one for Intel) and include them using the `configurable!` pseudo-macro.
///
/// ```rust
/// #[configurable]
/// mod kernels {
///
///     // You can mix external includes with inline definitions.
///     #[assumptions]
///     fn generic_kernel() {
///         println!("Running generic kernel");
///     }
/// 
///     // Looks for 'cuda_kernels.rs' in the same directory.
///     // That file should contain functions marked with #[assumptions(...)].
///     configurable!("cuda_kernels.rs");
/// }
/// ```
///
/// ## 3. Implementation Blocks & Traits
/// You can implement a standard Rust trait where the method implementation is chosen dynamically
/// based on hardware features.
///
/// ```rust
/// trait LinearAlgebra {
///     fn dot_product(&self, other: &Self) -> f32;
/// }
///
/// pub struct Vector { data: Vec<f32> }
///
/// #[configurable]
/// mod math_impls {
///     use super::{LinearAlgebra, Vector};
///
///     // Implement the trait. The macro will generate a dispatcher for `dot_product`.
///     impl LinearAlgebra for Vector {
///
///         // Variant C: Default / Fallback
///         #[assumptions]
///         fn dot_product(&self, other: &Self) -> f32 {
///             self.data.iter().zip(&other.data).map(|(a, b)| a * b).sum()
///         }
///
///         // Variant A: NVIDIA GPU Backend
///         // Conceptually: impl LinearAlgebra for Vector where Platform == CUDA
///         #[assumptions(acc_backend = CUDA)]
///         fn dot_product(&self, other: &Self) -> f32 {
///             println!("Dispatching to CUDA Kernel...");
///             0.0
///         }
///
///         // Variant B: CPU SIMD Optimization
///         // Conceptually: impl LinearAlgebra for Vector where Platform == AVX2
///         #[assumptions(cpu_simd = AVX2)]
///         fn dot_product(&self, other: &Self) -> f32 {
///             println!("Dispatching to AVX2 Intrinsic...");
///             0.0
///         }
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn configurable(_: TokenStream, item: TokenStream) -> TokenStream {
    __internal_configurable(item.into(), "configurable", "assumptions").into()
}
