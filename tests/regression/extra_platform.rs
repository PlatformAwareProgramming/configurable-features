impl MyTrait for MyDataType {
    #[assumptions(cpu_simd = AVX512, cpu_core_count = AtLeast { val: 8 })]
    fn compute(&self, x: i64) -> i64 {
        x + 200
    }
}
