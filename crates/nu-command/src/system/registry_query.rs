use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    record, Category, Example, IntoInterruptiblePipelineData, IntoPipelineData, PipelineData,
    ShellError, Signature, Span, Spanned, SyntaxShape, Type, Value,
};
use winreg::{enums::*, RegKey};

#[derive(Clone)]
pub struct RegistryQuery;

struct RegistryQueryArgs {
    hkcr: bool,
    hkcu: bool,
    hklm: bool,
    hku: bool,
    hkpd: bool,
    hkpt: bool,
    hkpnls: bool,
    hkcc: bool,
    hkdd: bool,
    hkculs: bool,
    key: String,
}

impl Command for RegistryQuery {
    fn name(&self) -> &str {
        "registry query"
    }

    fn signature(&self) -> Signature {
        Signature::build("registry query")
            .input_output_types(vec![(Type::Nothing, Type::Any)])
            .switch("hkcr", "query the hkey_classes_root hive", None)
            .switch("hkcu", "query the hkey_current_user hive", None)
            .switch("hklm", "query the hkey_local_machine hive", None)
            .switch("hku", "query the hkey_users hive", None)
            .switch("hkpd", "query the hkey_performance_data hive", None)
            .switch("hkpt", "query the hkey_performance_text hive", None)
            .switch("hkpnls", "query the hkey_performance_nls_text hive", None)
            .switch("hkcc", "query the hkey_current_config hive", None)
            .switch("hkdd", "query the hkey_dyn_data hive", None)
            .switch(
                "hkculs",
                "query the hkey_current_user_local_settings hive",
                None,
            )
            .required("key", SyntaxShape::String, "registry key to query")
            .optional(
                "value",
                SyntaxShape::String,
                "optionally supply a registry value to query",
            )
            .category(Category::System)
    }

    fn usage(&self) -> &str {
        "Query the Windows registry."
    }

    fn extra_usage(&self) -> &str {
        "Currently supported only on Windows systems."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        registry_query(engine_state, stack, call)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Query the HKEY_CURRENT_USER hive",
                example: "registry query --hkcu environment",
                result: None,
            },
            Example {
                description: "Query the HKEY_LOCAL_MACHINE hive",
                example: r"registry query --hklm 'SYSTEM\CurrentControlSet\Control\Session Manager\Environment'",
                result: None,
            },
        ]
    }
}

fn registry_query(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<PipelineData, ShellError> {
    let call_span = call.head;

    let registry_key: Spanned<String> = call.req(engine_state, stack, 0)?;
    let registry_key_span = &registry_key.clone().span;
    let registry_value: Option<Spanned<String>> = call.opt(engine_state, stack, 1)?;

    let reg_params = RegistryQueryArgs {
        hkcr: call.has_flag("hkcr"),
        hkcu: call.has_flag("hkcu"),
        hklm: call.has_flag("hklm"),
        hku: call.has_flag("hku"),
        hkpd: call.has_flag("hkpd"),
        hkpt: call.has_flag("hkpt"),
        hkpnls: call.has_flag("hkpnls"),
        hkcc: call.has_flag("hkcc"),
        hkdd: call.has_flag("hkdd"),
        hkculs: call.has_flag("hkculs"),
        key: registry_key.item,
    };

    let reg_key = get_reg_key(reg_params, call_span)?;

    if registry_value.is_none() {
        let mut reg_values = vec![];
        for (name, val) in reg_key.enum_values().flatten() {
            let (nu_value, reg_type) = reg_value_to_nu_value(val, call_span);
            reg_values.push(Value::record(
                record! {
                    "name" => Value::string(name, call_span),
                    "value" => nu_value,
                    "type" => Value::string(format!("{:?}", reg_type), call_span),
                },
                *registry_key_span,
            ))
        }
        Ok(reg_values.into_pipeline_data(engine_state.ctrlc.clone()))
    } else {
        match registry_value {
            Some(value) => {
                let reg_value = reg_key.get_raw_value(value.item.as_str());
                match reg_value {
                    Ok(val) => {
                        let (nu_value, reg_type) = reg_value_to_nu_value(val, call_span);
                        Ok(Value::record(
                            record! {
                                "name" => Value::string(value.item, call_span),
                                "value" => nu_value,
                                "type" => Value::string(format!("{:?}", reg_type), call_span),
                            },
                            value.span,
                        )
                        .into_pipeline_data())
                    }
                    Err(_) => Err(ShellError::GenericError(
                        "Unable to find registry key/value".to_string(),
                        format!("Registry value: {} was not found", value.item),
                        Some(value.span),
                        None,
                        Vec::new(),
                    )),
                }
            }
            None => Ok(Value::nothing(call_span).into_pipeline_data()),
        }
    }
}

fn get_reg_key(reg_params: RegistryQueryArgs, call_span: Span) -> Result<RegKey, ShellError> {
    let mut key_count = 0;
    let registry_key = if reg_params.hkcr {
        key_count += 1;
        RegKey::predef(HKEY_CLASSES_ROOT).open_subkey(reg_params.key)?
    } else if reg_params.hkcu {
        key_count += 1;
        RegKey::predef(HKEY_CURRENT_USER).open_subkey(reg_params.key)?
    } else if reg_params.hklm {
        key_count += 1;
        RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(reg_params.key)?
    } else if reg_params.hku {
        key_count += 1;
        RegKey::predef(HKEY_USERS).open_subkey(reg_params.key)?
    } else if reg_params.hkpd {
        key_count += 1;
        RegKey::predef(HKEY_PERFORMANCE_DATA).open_subkey(reg_params.key)?
    } else if reg_params.hkpt {
        key_count += 1;
        RegKey::predef(HKEY_PERFORMANCE_TEXT).open_subkey(reg_params.key)?
    } else if reg_params.hkpnls {
        key_count += 1;
        RegKey::predef(HKEY_PERFORMANCE_NLSTEXT).open_subkey(reg_params.key)?
    } else if reg_params.hkcc {
        key_count += 1;
        RegKey::predef(HKEY_CURRENT_CONFIG).open_subkey(reg_params.key)?
    } else if reg_params.hkdd {
        key_count += 1;
        RegKey::predef(HKEY_DYN_DATA).open_subkey(reg_params.key)?
    } else if reg_params.hkculs {
        key_count += 1;
        RegKey::predef(HKEY_CURRENT_USER_LOCAL_SETTINGS).open_subkey(reg_params.key)?
    } else {
        RegKey::predef(HKEY_CURRENT_USER).open_subkey(reg_params.key)?
    };

    if key_count > 1 {
        return Err(ShellError::GenericError(
            "Only one registry key can be specified".into(),
            "Only one registry key can be specified".into(),
            Some(call_span),
            None,
            Vec::new(),
        ));
    }
    Ok(registry_key)
}

fn clean_string(string: &str) -> String {
    string
        .trim_start_matches('"')
        .trim_end_matches('"')
        .replace("\\\\", "\\")
}
fn reg_value_to_nu_value(
    reg_value: winreg::RegValue,
    call_span: Span,
) -> (nu_protocol::Value, winreg::enums::RegType) {
    match reg_value.vtype {
        REG_NONE => (Value::nothing(call_span), reg_value.vtype),
        REG_SZ => (
            Value::string(clean_string(&reg_value.to_string()), call_span),
            reg_value.vtype,
        ),
        REG_EXPAND_SZ => (
            Value::string(clean_string(&reg_value.to_string()), call_span),
            reg_value.vtype,
        ),
        REG_BINARY => (Value::binary(reg_value.bytes, call_span), reg_value.vtype),
        REG_DWORD => (
            Value::int(
                unsafe { *(reg_value.bytes.as_ptr() as *const u32) } as i64,
                call_span,
            ),
            reg_value.vtype,
        ),
        REG_DWORD_BIG_ENDIAN => (
            Value::int(
                unsafe { *(reg_value.bytes.as_ptr() as *const u32) } as i64,
                call_span,
            ),
            reg_value.vtype,
        ),
        REG_LINK => (
            Value::string(clean_string(&reg_value.to_string()), call_span),
            reg_value.vtype,
        ),
        REG_MULTI_SZ => (
            Value::string(clean_string(&reg_value.to_string()), call_span),
            reg_value.vtype,
        ),
        REG_RESOURCE_LIST => (
            Value::string(clean_string(&reg_value.to_string()), call_span),
            reg_value.vtype,
        ),
        REG_FULL_RESOURCE_DESCRIPTOR => (
            Value::string(clean_string(&reg_value.to_string()), call_span),
            reg_value.vtype,
        ),
        REG_RESOURCE_REQUIREMENTS_LIST => (
            Value::string(clean_string(&reg_value.to_string()), call_span),
            reg_value.vtype,
        ),
        REG_QWORD => (
            Value::int(
                unsafe { *(reg_value.bytes.as_ptr() as *const u32) } as i64,
                call_span,
            ),
            reg_value.vtype,
        ),
    }
}
