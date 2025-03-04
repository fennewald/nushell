use nu_engine::env_to_strings;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Type, Value,
};

use super::utils::{gen_command, get_editor};

#[derive(Clone)]
pub struct ConfigNu;

impl Command for ConfigNu {
    fn name(&self) -> &str {
        "config nu"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .category(Category::Env)
            .input_output_types(vec![
                (Type::Nothing, Type::Nothing),
                (Type::Nothing, Type::String),
            ])
            .switch(
                "default",
                "Print default `config.nu` file instead.",
                Some('d'),
            )
        // TODO: Signature narrower than what run actually supports theoretically
    }

    fn usage(&self) -> &str {
        "Edit nu configurations."
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "allow user to open and update nu config",
                example: "config nu",
                result: None,
            },
            Example {
                description: "allow user to print default `config.nu` file",
                example: "config nu --default,",
                result: None,
            },
            Example {
                description: "allow saving the default `config.nu` locally",
                example: "config nu --default | save -f ~/.config/nushell/default_config.nu",
                result: None,
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        // `--default` flag handling
        if call.has_flag("default") {
            let head = call.head;
            return Ok(Value::string(nu_utils::get_default_config(), head).into_pipeline_data());
        }

        let env_vars_str = env_to_strings(engine_state, stack)?;
        let nu_config = match engine_state.get_config_path("config-path") {
            Some(path) => path.clone(),
            None => {
                return Err(ShellError::GenericError(
                    "Could not find $nu.config-path".to_string(),
                    "Could not find $nu.config-path".to_string(),
                    None,
                    None,
                    Vec::new(),
                ));
            }
        };

        let (item, config_args) = get_editor(engine_state, stack, call.head)?;

        gen_command(call.head, nu_config, item, config_args, env_vars_str).run_with_input(
            engine_state,
            stack,
            input,
            true,
        )
    }
}
