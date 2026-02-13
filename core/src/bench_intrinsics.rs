#[cfg(test)]
mod tests {
    use crate::intrinsics::intrinsic_add;
    use crate::runtime::Value;
    use std::time::Instant;

    #[test]
    fn benchmark_string_concat() {
        let iterations = 100_000;
        let base_str = "x".repeat(100); // 100 chars
        let append_str = "y".repeat(100);

        let start = Instant::now();

        for _ in 0..iterations {
            let a = Value::String(base_str.clone());
            let b = Value::String(append_str.clone());
            let args = vec![a, b];
            // intrinsic_add returns Result<Value, RuntimeError>
            let _res = intrinsic_add(args).unwrap();
        }

        let total_duration = start.elapsed();

        println!("Benchmark: {} iterations took {:?}", iterations, total_duration);
        println!("Average per op: {:?}", total_duration / iterations as u32);
    }
}
