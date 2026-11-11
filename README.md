# configurable-features

`configurable-features` is the foundational Rust crate for dynamically configurable systems upon which the [`platform-aware`](https://github.com/PlatformAwareProgramming/platform-aware) ecosystem is built. It provides the core abstractions, subtyping rules, runtime resolution algorithm, and procedural metaprogramming machinery introduced in the research on **Structured Platform-Aware Programming with Rust**.

---

## Conceptual Overview

Platform-aware programming is a performance engineering practice in which programmers explicitly encode assumptions about the characteristics of the target execution platform into the program to exploit hardware heterogeneity (such as SIMD instruction set extensions, multicore processors, and GPU accelerators like CUDA and ROCm).

To overcome the modularity and extensibility anomalies of *ad hoc* runtime feature detection in statically compiled languages without dynamic multiple dispatch, `configurable-features` implements **platform typing** (or configuration typing) at the value level combined with compile-time dispatch code generation:

1. **Features and Assumption Types**: Platform assumptions are modeled as types organized into hierarchical taxonomies that form a subtyping relation ($P <: Q$).
   - The root of each parameter taxonomy represents the **weakest assumption** ($\top$), applied by default when no argument is explicitly provided for that parameter.
   - Intermediate nodes represent sets of features corresponding to the union of their transitive successor nodes.
   - Leaf nodes (**singleton-feature assumptions**, or simply *features*) represent concrete platform characteristics.
2. **Qualifiers and Full Quantifiers**: Assumptions are classified into symbolic *qualifiers* (`QualifierFeature`) and refinement-inspired numerical *quantifiers* (`QuantifierFeature`).
3. **Kernel Modules, K-Functions, and K-Methods**: Modules annotated with `#[configurable]` (or `#[platformaware]` in `platform-aware`) encapsulate multiple versions of free functions (*k-functions*) or trait implementation methods (*k-methods*), each annotated with its assumption set via `#[assumptions(...)]` (or `#[kernelversion(...)]`).
4. **Addressing the Platform-Aware Expression Problem**: To separate platform-specific code along the platform decomposition axis ($\mathcal{P}$) without modifying existing `impl` blocks, variant implementations can be isolated into separate platform modules (inline submodules or external `<pmodule>.rs` / `<pmodule>/mod.rs` files) that are automatically woven together at compile time.

---

## Core Abstractions

### 1. The `Feature`, `QualifierFeature`, and `QuantifierFeature` Traits

All assumption types implement the [`Feature`](src/base/feature.rs) trait, which defines their canonical string representation and their immediate `supertype` in the assumption taxonomy:

```rust
pub enum FeatureKind {
    Qualifier,
    Quantifier,
}

pub trait Feature: Send + Sync {
    fn feature_obj(&self) -> FeatureObj;
    fn string(&self) -> String;
    fn supertype(&self) -> Option<Box<dyn Feature>> { None }
    fn hash_code(&self) -> u64 { /* ... */ }
}

pub trait QualifierFeature: Feature {
    fn feature_class(&self) -> PlatformParameter;
}

pub trait QuantifierFeature: Feature {
    fn quantifier_type(&self) -> QuantifierType;
    fn val(&self) -> i32;
}
```

### 2. Defining Qualifier Taxonomies (`create_feature_hierarchy!`)

Qualifiers are unit structs implementing `Feature` and `QualifierFeature`. Third-party crates or applications can define or extend assumption taxonomies for any parameter using the [`create_feature_hierarchy!`](src/create_feature_hierarchy.rs) macro, where `:>` denotes the supertype relation:

```rust
use configurable_features::create_feature_hierarchy;

// Chain syntax: CPUSIMD :> AVX :> AVX2 :> AVX512
create_feature_hierarchy! {
    register_simd; "cpu_simd" : None :> CPUSIMD :> AVX :> AVX2 :> AVX512;
}

// Branching leaves syntax: parent :> leaf1 & leaf2 & ...
create_feature_hierarchy! {
    register_acc_root; "acc_model" : None :> ACCModel;
}
create_feature_hierarchy! {
    register_acc_vendor; "acc_model" : ACCModel :> NVIDIA_GPU & AMD_GPU & Intel_GPU;
}
create_feature_hierarchy! {
    register_acc_arch; "acc_model" : NVIDIA_GPU :> NVIDIA_GPU_Blackwell & NVIDIA_GPU_Ada & NVIDIA_GPU_Hopper;
}
create_feature_hierarchy! {
    register_acc_model; "acc_model" : NVIDIA_GPU_Ada :> NVIDIA_GPU_A100 & NVIDIA_GPU_A200 & NVIDIA_GPU_RTX4090 & NVIDIA_GPU_RTX4000;
}
```

This establishes the transitive subtyping relation (e.g., `AVX512 <: AVX2 <: AVX <: CPUSIMD` and `NVIDIA_GPU_A100 <: NVIDIA_GPU_Ada <: NVIDIA_GPU <: ACCModel`) and automatically registers the qualifiers in the global feature catalog at startup.

### 3. Full Quantifiers

Quantifiers express assumptions over numerically valued parameters (`i32`). Three kinds of quantifiers are supported in [`src/base/quantifiers.rs`](src/base/quantifiers.rs):

- **Exact-value quantifier** (`n: i32`): denotes the singleton set $\{n\}$.
- **At-least-value quantifier** (`AtLeast { val: n }`): denotes $\{v \mid v \in \text{i32} \land v \ge n\}$.
- **At-most-value quantifier** (`AtMost { val: n }`): denotes $\{v \mid v \in \text{i32} \land v \le n\}$.

Inspired by refinement types, the following subtyping rules hold:
- $\text{AtLeast}\{\text{val}: m\} <: \text{AtLeast}\{\text{val}: n\} \iff m \ge n$
- $\text{AtMost}\{\text{val}: m\} <: \text{AtMost}\{\text{val}: n\} \iff m \le n$
- $m <: \text{AtLeast}\{\text{val}: n\} \iff m \ge n$
- $m <: \text{AtMost}\{\text{val}: n\} \iff m \le n$

---

## Kernel Modules, K-Functions, and K-Methods

Inside a `#[configurable]` module, multiple variants of a function or trait method can coexist under different `#[assumptions(...)]` attributes. The version declared without assumptions is the **fallback version** (platform type $\top$), which is guaranteed to be compatible with any execution platform.

### Standalone K-Functions

```rust
use configurable_features::{AtLeast, AtMost, configurable};

#[configurable]
mod my_kernel_module {
    use super::*;

    #[assumptions] // v0: fallback version (no platform assumptions)
    pub fn my_kernel_function(x: i64, y: i64, z: i64) -> i64 {
        x + y + z
    }

    #[assumptions(cpu_simd = AVX512, cpu_core_count = AtMost { val: 4 })] // v1
    pub fn my_kernel_function(x: i64, y: i64, z: i64) -> i64 {
        x + y + z
    }

    #[assumptions(cpu_simd = AVX512, cpu_core_count = AtLeast { val: 8 })] // v2
    pub fn my_kernel_function(x: i64, y: i64, z: i64) -> i64 {
        x + y + z
    }

    #[assumptions(acc_count = AtLeast { val: 2 }, acc_backend = CUDA, acc_cudacc = AtLeast { val: 90 })] // v4
    pub fn my_kernel_function(x: i64, y: i64, z: i64) -> i64 {
        x + y + z
    }

    #[assumptions(acc_backend = CUDA, acc_cudacc = AtLeast { val: 80 })] // v5
    pub fn my_kernel_function(x: i64, y: i64, z: i64) -> i64 {
        x + y + z
    }
}
```

Callers invoke `my_kernel_module::my_kernel_function(1, 2, 3)` as an ordinary Rust function, completely unaware of its multi-variant nature.

### K-Methods and Platform Modules

To address the **Platform-Aware Expression Problem**, trait implementations containing *k-methods* can split platform-specific variants across inline submodules or external files (`<pmodule>.rs` or `<pmodule>/mod.rs`):

```rust
pub trait MyTrait {
    fn my_kernel_function(&self, x: i64, y: i64, z: i64) -> i64;
}

pub struct MyDataType;

#[configurable]
mod my_kernel_module {
    use super::{MyDataType, MyTrait};

    impl MyTrait for MyDataType {
        #[assumptions] // fallback version (v0)
        fn my_kernel_function(&self, x: i64, y: i64, z: i64) -> i64 {
            x + y + z
        }
    }

    // Inline platform submodule
    #[configurable]
    mod cpu_simd {
        use super::*;
        impl MyTrait for MyDataType {
            #[assumptions(cpu_simd = AVX512, cpu_core_count = AtLeast { val: 8 })]
            fn my_kernel_function(&self, x: i64, y: i64, z: i64) -> i64 {
                x + y + z
            }
        }
    }

    // Or external file inclusion (loads `gpu_cuda.rs` or `gpu_cuda/mod.rs`)
    configurable!(gpu_cuda);
}
```

The procedural macro merges matching `impl` blocks, renames each variant to `<name>_variant_<i>`, and generates a dispatcher method that caches the selected index in a lazily initialized `static SELECTED_VARIANT: LazyLock<i32>`.

---

## The Resolution Algorithm

Let $P$ be the **actual platform feature set** and $Q$ be a platform type. $P$ is a subtype of $Q$ ($P <: Q$) if, for every registered platform parameter, the assumption in $P$ is a subtype of the corresponding assumption in $Q$.

Upon the first invocation of a *k-function* or *k-method*, the [`resolve`](src/resolve/mod.rs) function selects the best matching variant from `candidates[0..=n]`:

1. Initialize the current selection $C \leftarrow \top$ and $i \leftarrow n$ (the last declared candidate).
2. While $i \ge 0$:
   - Find the greatest index $j \in [0, i]$ such that $P <: \text{candidates}[j] <: C$.
   - If found ($j \ge 0$), update $C \leftarrow \text{candidates}[j]$ and $i \leftarrow j - 1$.
3. Return the index corresponding to $C$.

By scanning in reverse declaration order and refining the selection along a chain $P <: K_n <: \dots <: K_1 <: K_0 <: \top$:
- The **strongest assumption** ($K_n$) satisfied by the actual platform feature set $P$ is selected.
- When two compatible candidates have **overlapping (incomparable) assumptions**, the one declared later takes precedence, giving programmers deterministic control without ambiguity errors.

---

## Platform Description File (`Platform.toml`)

The actual platform feature set ($P$) can be specified via a human-editable `Platform.toml` file loaded from the directory pointed to by the `PLATFORM_DESCRIPTION` environment variable (or the current working directory by default):

```toml
[processor]
cpu_count = 1
cpu_core_count = 32
cpu_threads_count = 64
cpu_model = "AMD_EPYC_7513"
cpu_microarchitecture = "Milan"
cpu_isa = "ISA_x86_64_v3"
cpu_simd = ["MMX+", "SSE_4_2", "SSE_4a", "AVX2"]
cpu_L1_size = 65536
cpu_L2_size = 524288
cpu_L3_size = 134217728
cpu_core_clock = 2600
cpu_tdp = 200

[accelerator]
acc_count = 1
acc_type = "GPU"
acc_backend = ["CUDA_12_0", "Vulkan_1_3", "OpenGL_4_6", "OpenCL_3_0", "DirectX_12_0"]
acc_model = "NVIDIA_A100_80GB"
acc_processor = "GA100"
acc_processorcount = 6912
acc_memorytype = "HBM2e"
acc_memorysize = 80
acc_interconnect = "PCIe_4_0_x16"
acc_tdp = 300
acc_cudacc = 80
acc_cudatoolkit = 110
acc_cudadriver = 11
```

---

## Workspace Structure

- **`configurable-features`** ([`src/`](src/)): Runtime traits (`Feature`, `QualifierFeature`, `QuantifierFeature`), quantifiers (`AtLeast`, `AtMost`, `i32`), `create_feature_hierarchy!` macro, `Platform.toml` reader, `PlatformRegistry`, and the resolution algorithm (`resolve`, `issubtypeof`).
- **`configurable-macros`** ([`configurable-macros/`](configurable-macros/)): Procedural macro crate exposing the `#[configurable]` attribute.
- **`configurable-internal`** ([`configurable-internal/`](configurable-internal/)): Reusable code-generation engine (`__internal_configurable`) shared with downstream crates such as `platform-aware` (which parameterizes it with `#[platformaware]`, `#[kernelversion]`, and `platform!`).

---

## Building and Testing

```bash
cargo build --workspace --all-targets
cargo test --workspace
```

---

## References

- F. H. de Carvalho-Junior, J. M. Alves Nogueira, J. M. Uchôa de Alencar. *Structured platform-aware programming with Rust*.
- F. H. de Carvalho-Junior, J. M. A. Nogueira, J. M. U. d. Alencar. *Structured Platform-Aware Programming for Rust*. In: XXVIII Brazilian Symposium on Programming Languages (SBLP 2025), SBC, Porto Alegre, Brazil, 2025.
