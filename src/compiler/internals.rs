use std::sync::OnceLock;

use crate::error::Location;

use super::{file_tree::ResourceLocation, Compiler};

static RESET_DIRECT_RETURN: OnceLock<ResourceLocation> = OnceLock::new();
static DYNAMIC_INDEX: OnceLock<ResourceLocation> = OnceLock::new();
static DYNAMIC_RANGE_INDEX: OnceLock<ResourceLocation> = OnceLock::new();
static DYNAMIC_RANGE_INDEX_NO_END: OnceLock<ResourceLocation> = OnceLock::new();
static DYNAMIC_MEMBER: OnceLock<ResourceLocation> = OnceLock::new();

const VERSION: &str = env!("CARGO_PKG_VERSION");

impl Compiler {
  pub fn reset_direct_return(&mut self) -> &ResourceLocation {
    RESET_DIRECT_RETURN.get_or_init(|| {
      let location = ResourceLocation::new_function(
        "zoglin", &["internal", VERSION,
        "reset_return"],
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

  pub fn dynamic_index(&mut self) -> &ResourceLocation {
    DYNAMIC_INDEX.get_or_init(|| {
      let location = ResourceLocation::new_function(
        "zoglin", &["internal", VERSION,
        "dynamic_index"],
      );
      self
        .add_function_item(
          Location::blank(),
          location.clone(),
          vec![
            format!(
              "$data modify storage zoglin:internal/{VERSION}/dynamic_index return set from storage zoglin:internal/{VERSION}/dynamic_index target[$(__index)]"
            ),
          ],
        )
        .expect("Function should not already be defined");
      location
    })
  }

  pub fn dynamic_range_index(&mut self) -> &ResourceLocation {
    DYNAMIC_RANGE_INDEX.get_or_init(|| {
      let location = ResourceLocation::new_function(
        "zoglin", &["internal", VERSION,
        "dynamic_range_index"],
      );
      self
        .add_function_item(
          Location::blank(),
          location.clone(),
          vec![
            format!(
              "$data modify storage zoglin:internal/{VERSION}/dynamic_range_index return set string storage zoglin:internal/{VERSION}/dynamic_range_index target $(__start) $(__end)"
            ),
          ],
        )
        .expect("Function should not already be defined");
      location
    })
  }

  pub fn dynamic_range_index_no_end(&mut self) -> &ResourceLocation {
    DYNAMIC_RANGE_INDEX_NO_END.get_or_init(|| {
      let location = ResourceLocation::new_function(
        "zoglin", &["internal", VERSION,
        "dynamic_range_index_no_end"],
      );
      self
        .add_function_item(
          Location::blank(),
          location.clone(),
          vec![
            format!(
              "$data modify storage zoglin:internal/{VERSION}/dynamic_range_index_no_end return set string storage zoglin:internal/{VERSION}/dynamic_range_index_no_end target $(__start)"
            ),
          ],
        )
        .expect("Function should not already be defined");
      location
    })
  }

  pub fn dynamic_member(&mut self) -> &ResourceLocation {
    DYNAMIC_MEMBER.get_or_init(|| {
      let location = ResourceLocation::new_function(
        "zoglin", &["internal", VERSION,
        "dynamic_member"],
      );
      self
        .add_function_item(
          Location::blank(),
          location.clone(),
          vec![
            format!(
              "$data modify storage zoglin:internal/{VERSION}/dynamic_member return set from storage zoglin:internal/{VERSION}/dynamic_member target.\"$(__member)\""
            ),
          ],
        )
        .expect("Function should not already be defined");
      location
    })
  }
}
