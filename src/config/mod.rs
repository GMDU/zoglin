#![allow(dead_code)]

mod serde_impl;
use std::{cmp::Ordering, collections::HashMap, fmt::Display};

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
  /// The zoglin version constraint
  pub zoglin: VersionConstraint,
  /// The entrypoint for the program
  pub entry: String,
  /// Information about the package itself
  pub package: Package,
  /// The pack.mcmeta file to generate
  pub meta: McMeta,
  /// The map of dependency projects to their version constraints
  pub dependencies: HashMap<String, VersionConstraint>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Package {
  /// The name of the package
  pub name: String,
  /// The current package version
  pub version: Version,
  /// A short summary of the package
  pub summary: String,
  /// The name of the author of this package
  pub author: String,
  /// The version constraint for supported Minecraft versions
  pub supports: VersionConstraint,
  /// The contact for the author of this package
  #[serde(skip_serializing_if = "Option::is_none")]
  #[serde(default)]
  pub contact: Option<String>,
  /// The main web page for this package
  #[serde(skip_serializing_if = "Option::is_none")]
  #[serde(default)]
  pub homepage: Option<String>,
  /// The link to the source code of this package
  #[serde(skip_serializing_if = "Option::is_none")]
  #[serde(default)]
  pub source: Option<String>,
  /// The link to the place to report issues for this package
  #[serde(skip_serializing_if = "Option::is_none")]
  #[serde(default)]
  pub issues: Option<String>,
  /// The license of this package
  #[serde(skip_serializing_if = "Option::is_none")]
  #[serde(default)]
  pub license: Option<License>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct McMeta {
  pub pack: McPack,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct McPack {
  pub pack_format: u32,
  pub description: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  #[serde(default)]
  pub supported_formats: Option<SupportedFormats>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SupportedFormats {
  Single(u32),
  ArrayRange([u32; 2]),
  ObjectRange {
    min_inclusive: u32,
    max_inclusive: u32,
  },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct License {
  pub name: String,
  pub url: String,
}

#[derive(Debug)]
pub struct Version {
  pub major: u32,
  pub minor: Option<u32>,
  pub patch: Option<u32>,
  pub extra: Option<String>,
}

impl Version {
  fn next(&self) -> Version {
    match self {
      Version {
        major,
        minor,
        patch: None,
        extra: None,
      } => Version {
        major: *major + 1,
        minor: *minor,
        patch: None,
        extra: None,
      },
      Version {
        major,
        minor: Some(minor),
        patch: patch @ Some(_),
        extra: None,
      } => Version {
        major: *major,
        minor: Some(minor + 1),
        patch: *patch,
        extra: None,
      },
      Version {
        major: _,
        minor: None,
        patch: Some(_),
        extra: None,
      } => unreachable!("A version with a patch and no minor is not possible"),
      Version {
        major,
        minor,
        patch,
        extra: Some(_),
      } => Version {
        major: *major,
        minor: *minor,
        patch: *patch,
        extra: None,
      },
    }
  }
}

impl Display for Version {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.major)?;
    if let Some(minor) = self.minor {
      write!(f, ".{minor}")?;
    }
    if let Some(patch) = self.patch {
      write!(f, ".{patch}")?;
    }
    if let Some(extra) = &self.extra {
      write!(f, "{extra}")?;
    }
    Ok(())
  }
}

impl PartialEq for Version {
  fn eq(&self, other: &Self) -> bool {
    self.major == other.major
      && self.minor.unwrap_or(0) == other.minor.unwrap_or(0)
      && self.patch.unwrap_or(0) == other.patch.unwrap_or(0)
      && self.extra == other.extra
  }
}

impl PartialOrd for Version {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    match self.major.partial_cmp(&other.major) {
      Some(Ordering::Equal) => {}
      ord => return ord,
    }
    match self
      .minor
      .unwrap_or(0)
      .partial_cmp(&other.minor.unwrap_or(0))
    {
      Some(Ordering::Equal) => {}
      ord => return ord,
    }
    match self
      .patch
      .unwrap_or(0)
      .partial_cmp(&other.patch.unwrap_or(0))
    {
      Some(Ordering::Equal) => {}
      ord => return ord,
    }
    match (&self.extra, &other.extra) {
      (None, None) => Some(Ordering::Equal),
      (None, Some(_)) => Some(Ordering::Greater),
      (Some(_), None) => Some(Ordering::Less),
      (Some(_), Some(_)) => None,
    }
  }
}

#[derive(Debug)]
pub enum VersionConstraint {
  Single {
    kind: ConstraintKind,
    constrained_to: Version,
  },
  Grouped(Box<VersionConstraint>),
  And(Box<VersionConstraint>, Box<VersionConstraint>),
  Or(Box<VersionConstraint>, Box<VersionConstraint>),
}

impl VersionConstraint {
  fn matches(&self, v: &Version) -> bool {
    match self {
      VersionConstraint::Single {
        kind,
        constrained_to,
      } => match kind {
        ConstraintKind::Single => v == constrained_to,
        ConstraintKind::Greater => v > constrained_to,
        ConstraintKind::GreaterOrEqual => v >= constrained_to,
        ConstraintKind::Less => v < constrained_to,
        ConstraintKind::LessOrEqual => v <= constrained_to,
        ConstraintKind::UntilNextSignificant => v >= constrained_to && v < &constrained_to.next(),
      },
      VersionConstraint::Grouped(version_constraint) => version_constraint.matches(v),
      VersionConstraint::And(a, b) => a.matches(v) && b.matches(v),
      VersionConstraint::Or(a, b) => a.matches(v) || b.matches(v),
    }
  }
}

impl Display for VersionConstraint {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      VersionConstraint::Single {
        kind,
        constrained_to,
      } => write!(f, "{}{}", kind.as_str(), constrained_to),
      VersionConstraint::Grouped(inner) => {
        write!(f, "(")?;
        inner.fmt(f)?;
        write!(f, ")")
      }
      VersionConstraint::And(a, b) => {
        a.fmt(f)?;
        write!(f, " && ")?;
        b.fmt(f)
      }
      VersionConstraint::Or(a, b) => {
        a.fmt(f)?;
        write!(f, " || ")?;
        b.fmt(f)
      }
    }
  }
}

#[derive(Debug)]
pub enum ConstraintKind {
  Single,
  Greater,
  GreaterOrEqual,
  Less,
  LessOrEqual,
  UntilNextSignificant,
}

impl ConstraintKind {
  fn as_str(&self) -> &'static str {
    match self {
      ConstraintKind::Single => "",
      ConstraintKind::Greater => "> ",
      ConstraintKind::GreaterOrEqual => ">= ",
      ConstraintKind::Less => "< ",
      ConstraintKind::LessOrEqual => "<= ",
      ConstraintKind::UntilNextSignificant => "~> ",
    }
  }
}
