use std::sync::OnceLock;

use crate::error::Location;

use super::{
  file_tree::{FunctionLocation, ResourceLocation},
  Compiler,
};

static RESET_DIRECT_RETURN: OnceLock<FunctionLocation> = OnceLock::new();
static DYNAMIC_INDEX: OnceLock<FunctionLocation> = OnceLock::new();
const VERSION: &str = env!("CARGO_PKG_VERSION");

impl Compiler {
  pub(super) fn reset_direct_return(&mut self) -> &FunctionLocation {
    RESET_DIRECT_RETURN.get_or_init(|| {
      let location = FunctionLocation::new(
        ResourceLocation::new("zoglin", &["internal", VERSION]),
        "reset_return",
      );
      self
        .add_function_item(
          Location::blank(),
          location.clone(),
          Vec::from([
            "scoreboard players operation $temp_return zoglin.internal.vars = $should_return zoglin.internal.vars",
            "scoreboard players reset $should_return zoglin.internal.vars", 
            "return run scoreboard players get $temp_return zoglin.internal.vars"
          ].map(|s| s.to_string())),
        )
        .expect("Function should not already be defined");
      location
    })
  }

  pub(super) fn dynamic_index(&mut self) -> &FunctionLocation {
    DYNAMIC_INDEX.get_or_init(|| {
      let location = FunctionLocation::new(
        ResourceLocation::new("zoglin", &["internal", VERSION]),
        "dynamic_index",
      );
      self
        .add_function_item(
          Location::blank(),
          location.clone(),
          vec![
            format!(
              "$data modify storage zoglin:internal/{VERSION}/dynamic_index return set from storage zoglin:internal/{VERSION}/dynamic_index target[$(index)]"
            ),
          ],
        )
        .expect("Function should not already be defined");
      location
    })
  }
}
