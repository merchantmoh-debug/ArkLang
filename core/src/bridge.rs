/*
 * Copyright (c) 2026 Mohamad Al-Zawahreh (dba Sovereign Systems).
 *
 * This file is part of the Ark Sovereign Compiler.
 *
 * LICENSE: DUAL-LICENSED (AGPLv3 or COMMERCIAL).
 *
 * 1. OPEN SOURCE: You may use this file under the terms of the GNU Affero
 * General Public License v3.0. If you link to this code, your ENTIRE
 * application must be open-sourced under AGPLv3.
 *
 * 2. COMMERCIAL: For proprietary use, you must obtain a Commercial License
 * from Sovereign Systems.
 *
 * PATENT NOTICE: Protected by US Patent App #63/935,467.
 * NO IMPLIED LICENSE to rights of Mohamad Al-Zawahreh or Sovereign Systems.
 */

use crate::ast::FunctionDef;
use arrow::array::{Int32Array, StringArray, StructArray};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::ipc::writer::FileWriter;
use arrow::record_batch::RecordBatch;
use std::fs::File;
use std::sync::Arc;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum BridgeError {
    #[error("Arrow error: {0}")]
    Arrow(#[from] arrow::error::ArrowError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub fn write_dummy_state(path: &str) -> Result<(), BridgeError> {
    // Define Schema:
    // State {
    //   id: Utf8,
    //   value: Utf8
    // }
    let schema = Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("value", DataType::Utf8, false),
    ]);
    let schema_ref = Arc::new(schema);

    // Create Dummy Data (e.g., active linear variables)
    let ids = StringArray::from(vec!["var1", "var2", "resource_A"]);
    let values = StringArray::from(vec!["Linear", "Affine", "Linear"]);

    let batch = RecordBatch::try_new(schema_ref.clone(), vec![Arc::new(ids), Arc::new(values)])?;

    // Write to IPC File
    let file = File::create(path)?;
    let mut writer = FileWriter::try_new(file, &schema_ref)?;
    writer.write(&batch)?;
    writer.finish()?;

    Ok(())
}
