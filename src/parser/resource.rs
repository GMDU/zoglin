use ecow::EcoString;

use crate::error::Result;
use crate::lexer::token::TokenKind;

use super::{
  ast::{ImportPath, ZoglinResource},
  name::{validate, validate_zoglin_resource, NameKind},
  Parser,
};

impl Parser {
  pub fn parse_zoglin_resource(&mut self, kind: NameKind) -> Result<ZoglinResource> {
    let mut resource = ZoglinResource {
      namespace: None,
      location: self.current().location.clone(),
      modules: Vec::new(),
      name: EcoString::new(),
    };
    let mut allow_colon: bool = true;
    if self.current().kind == TokenKind::Colon {
      self.consume();
      allow_colon = false;
      resource.namespace = Some(EcoString::new());
    } else if self.current().kind == TokenKind::Tilde {
      self.consume();
      allow_colon = false;
      resource.namespace = Some("~".into());
      if self.current().kind == TokenKind::ForwardSlash {
        self.consume();
      }
    }
    loop {
      let identifier = self.expect(TokenKind::Identifier)?.value.clone();
      match self.current().kind {
        TokenKind::Colon => {
          self.consume();
          if allow_colon && self.current().kind == TokenKind::Identifier {
            resource.namespace = Some(identifier);
            allow_colon = false;
          } else {
            resource.name = identifier;
            break;
          }
        }
        TokenKind::ForwardSlash => {
          resource.modules.push(identifier);
          allow_colon = false;
          self.consume();
        }
        _ => {
          resource.name = identifier;
          break;
        }
      }
    }
    validate_zoglin_resource(&resource, kind)?;
    Ok(resource)
  }

  pub fn parse_import_resource(&mut self) -> Result<ImportPath> {
    let mut path = Vec::new();
    let mut is_comptime = false;
    let namespace = self.expect(TokenKind::Identifier)?;
    validate(&namespace.value, &namespace.location, NameKind::Namespace)?;
    let namespace = namespace.value.clone();
    self.expect(TokenKind::Colon)?;

    loop {
      if self.current().kind == TokenKind::Ampersand {
        self.consume();
        is_comptime = true;
        let name = self.expect(TokenKind::Identifier)?.value.clone();
        path.push(name);
        break;
      }

      let identifier = self.expect(TokenKind::Identifier)?.clone();
      if self.current().kind == TokenKind::ForwardSlash {
        validate(&identifier.value, &identifier.location, NameKind::Module)?;
        path.push(identifier.value.clone());
        self.consume();
      } else {
        validate(
          &identifier.value,
          &identifier.location,
          NameKind::ResourcePathComponent,
        )?;
        path.push(identifier.value.clone());
        break;
      }
    }

    Ok(ImportPath {
      namespace,
      path,
      is_comptime,
    })
  }
}
