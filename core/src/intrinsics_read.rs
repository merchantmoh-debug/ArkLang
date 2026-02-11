fn intrinsic_fs_read(args: Vec<Value>) -> Result<Value, EvalError> {
    if args.len() != 1 {
        return Err(EvalError::NotExecutable);
    }
    let path_str = match &args[0] {
        Value::String(s) => s,
        _ => {
            return Err(EvalError::TypeMismatch(
                "String".to_string(),
                args[0].clone(),
            ))
        }
    };

    println!("[Ark:FS] Reading from {}", path_str);
    let content = fs::read_to_string(path_str).map_err(|_| EvalError::NotExecutable)?;
    Ok(Value::String(content))
}
