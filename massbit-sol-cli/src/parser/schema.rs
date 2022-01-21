use crate::config::IndexerConfig;
use crate::parser::visitor::Visitor;
use crate::parser::Definitions;
use inflector::Inflector;
use std::fmt::Write;
use syn::__private::ToTokens;
use syn::{Fields, FieldsNamed, FieldsUnnamed, File, Item, ItemEnum, ItemUse, Type, Variant};

pub struct GraphqlSchema<'a> {
    pub config: IndexerConfig,
    definitions: &'a Definitions,
    entity_types: Vec<String>,
    current_entity_fields: Vec<String>,
}

impl<'a> GraphqlSchema<'a> {
    pub fn new(config: IndexerConfig, definitions: &'a Definitions) -> Self {
        Self {
            config,
            definitions,
            entity_types: Vec::default(),
            current_entity_fields: vec![],
        }
    }
}

impl<'a> Visitor for GraphqlSchema<'a> {
    fn visit_item_enum(&mut self, item_enum: &ItemEnum) {
        let ident = item_enum.ident.to_string();
        if self.config.main_instruction.as_str() == ident.as_str() {
            // for attr in item_enum.attrs.iter() {
            //     println!("{:?}", attr);
            //     println!("{:?}", attr.to_token_stream().to_string());
            // }
            item_enum
                .variants
                .iter()
                .for_each(|variant| self.visit_item_variant(item_enum, variant));
        }
    }
    fn visit_item_variant(&mut self, item_enum: &ItemEnum, variant: &Variant) {
        let ident_name = variant.ident.to_string();
        match &variant.fields {
            Fields::Named(named_field) => {
                self.visit_named_field(&ident_name, named_field);
            }
            Fields::Unnamed(fields_unnamed) => {
                self.visit_unnamed_field(&ident_name, fields_unnamed);
            }
            Fields::Unit => self.visit_unit_field(&ident_name),
        }
        let entity = format!(
            r#"type {entity_name} @entity {{
    id: ID!,
    block_timestamp: BigInt!,
    tx_hash: String
    {fields}
}}"#,
            entity_name = ident_name,
            fields = self.current_entity_fields.join(",\n\t")
        );
        self.entity_types.push(entity);
        //Clear current entity fields
        self.current_entity_fields = Vec::default();
    }
    fn visit_item_use(&mut self, item_use: &ItemUse) {}

    fn visit_named_field(&mut self, ident_name: &String, field_named: &FieldsNamed) {
        self.current_entity_fields = if let Some(field) = field_named.named.first() {
            let item_def = field.ty.to_token_stream().to_string();
            self.definitions
                .get_item_def(&item_def)
                .map(|item_def| item_def.create_entity_fields(self.definitions))
                .unwrap_or_default()
        } else {
            Vec::default()
        };
    }

    fn visit_unnamed_field(&mut self, ident_name: &String, field_unnamed: &FieldsUnnamed) {
        self.current_entity_fields = if let Some(field) = field_unnamed.unnamed.first() {
            let item_def = field.ty.to_token_stream().to_string();
            self.definitions
                .get_item_def(&item_def)
                .map(|item_def| item_def.create_entity_fields(self.definitions))
                .unwrap_or_default()
        } else {
            Vec::default()
        };
    }

    fn visit_unit_field(&mut self, ident_name: &String) {}

    fn create_content(&self) -> String {
        self.entity_types.join("\n")
    }

    fn create_dir_path(&self) -> String {
        format!("{}/src", self.config.output_logic)
    }
}
