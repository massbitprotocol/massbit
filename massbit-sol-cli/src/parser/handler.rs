use super::visitor::Visitor;
use super::Definitions;
use crate::config::IndexerConfig;
use crate::schema::AccountInfo;
use inflector::Inflector;
use std::collections::HashMap;
use std::fmt::Write;
use syn::__private::ToTokens;
use syn::{Fields, FieldsNamed, FieldsUnnamed, File, Item, ItemEnum, ItemUse, Variant};

const MODULES: &str = r#"
use crate::STORE;
use massbit_solana_sdk::entity::{Attribute, Entity, Value};
use massbit_solana_sdk::{
    transport::{TransportValue, Value as TransValue},
    types::SolanaBlock
};
use serde_json;
use solana_program::pubkey::Pubkey;
use solana_transaction_status::TransactionWithStatusMeta;
use std::collections::HashMap;
use uuid::Uuid;
"#;
const ENTITY_SAVE: &str = r#"
pub trait TransportValueExt {
    fn save(&self);
}
impl TransportValueExt for TransportValue {
    fn save(&self) {
        unsafe {
            STORE
                .as_mut()
                .unwrap()
                .save_values(&self.name, &self.values);
        }
    }
}
"#;
pub struct InstructionHandler<'a> {
    pub config: IndexerConfig,
    definitions: &'a Definitions,
    patterns: Vec<String>,
    handler_functions: Vec<String>,
    variant_accounts: &'a HashMap<String, Vec<AccountInfo>>,
}

impl<'a> InstructionHandler<'a> {
    pub fn new(
        config: IndexerConfig,
        definitions: &'a Definitions,
        variant_accounts: &'a HashMap<String, Vec<AccountInfo>>,
    ) -> Self {
        Self {
            config,
            definitions,
            patterns: vec![],
            handler_functions: vec![],
            variant_accounts,
        }
    }
}
impl<'a> Visitor for InstructionHandler<'a> {
    fn visit_item_enum(&mut self, item_enum: &ItemEnum) {
        let ident = item_enum.ident.to_string();
        if self.config.main_instruction.as_str() == ident.as_str() {
            //Document and derive
            // for attr in item_enum.attrs.iter() {
            //     println!("{:?}", attr);
            //     println!("{:?}", attr.to_token_stream().to_string());
            // }
            println!(
                "Enum name {:?}, Variant number: {}",
                item_enum.ident,
                item_enum.variants.len()
            );
            item_enum
                .variants
                .iter()
                .for_each(|variant| self.visit_item_variant(item_enum, variant));
        }
    }
    fn visit_item_variant(&mut self, item_enum: &ItemEnum, variant: &Variant) {
        let ident_name = variant.ident.to_string();
        let ident_snake = ident_name.to_snake_case();
        let pattern = format!(
            r#""{}" => {{self.process_{}(block, transaction, program_id, accounts, &mut input);}}"#,
            ident_name, &ident_snake
        );
        self.patterns.push(pattern);
        match &variant.fields {
            Fields::Named(named_field) => {
                self.visit_named_field(&ident_name, named_field);
            }
            Fields::Unnamed(fields_unnamed) => {
                self.visit_unnamed_field(&ident_name, fields_unnamed);
            }
            Fields::Unit => self.visit_unit_field(&ident_name),
        }
        //Generate process function
        let account_values = self.variant_accounts.get(&ident_name).map(|acc_infos| {
            acc_infos
                .iter()
                .map(|info| {
                    format!(
                        r#"input.set_value("{account_name}", TransValue::from(accounts.get({acc_index}).map(|acc|acc.to_string())));"#,
                        account_name = &info.name,
                        acc_index = info.index
                    )
                })
                .collect::<Vec<String>>()
        }).unwrap_or_default();
        let function = format!(
            r#"fn process_{fn_name}(
                &self,
                block: &SolanaBlock,
                transaction: &TransactionWithStatusMeta,
                program_id: &Pubkey,
                accounts: &Vec<Pubkey>,
                input: &mut TransportValue,
            ) -> Result<(), anyhow::Error> {{
                println!(
                    "call function process_initialize for handle incoming block {{}} with argument {{:?}}",
                    block.block_number, &input.name
                );
                input.set_value("block_timestamp", TransValue::from(block.timestamp));
                input.set_value("tx_hash", TransValue::from(transaction.transaction.signatures.iter().map(|sig| sig.to_string()).collect::<Vec<String>>().join(",'")));
                {account_values}
                input.save();
                println!("Write to db {{:?}}",input);
                Ok(())
            }}"#,
            fn_name = ident_snake,
            account_values = account_values.join("")
        );
        self.handler_functions.push(function);
    }
    fn visit_item_use(&mut self, item_use: &ItemUse) {}
    fn visit_named_field(&mut self, ident_name: &String, field_named: &FieldsNamed) {}
    fn visit_unnamed_field(&mut self, ident_name: &String, field_unnamed: &FieldsUnnamed) {}
    fn visit_unit_field(&mut self, ident_name: &String) {}
    fn create_content(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(&mut out, "{}", MODULES);
        let _ = writeln!(&mut out, "{}", ENTITY_SAVE);
        let _ = writeln!(
            &mut out,
            r#"pub struct Handler {{}}
                    impl Handler {{
                        pub fn process(
                            &self,
                            block: &SolanaBlock,
                            transaction: &TransactionWithStatusMeta,
                            program_id: &Pubkey,
                            accounts: &Vec<Pubkey>,
                            mut input: TransportValue,
                        ) {{
                            //println!("Process block {{}} with input {{:?}}", block.block_number, input);                           
                            match input.name.as_str() {{
                                {patterns}
                                _ => {{}}
                            }}
                            
                        }}
                        {handler_functions}
                    }}"#,
            patterns = self.patterns.join("\n"),
            handler_functions = self.handler_functions.join("\n")
        );
        out
    }
    fn create_dir_path(&self) -> String {
        format!("{}/src/generated", self.config.output_logic)
    }

    fn build(&self) {
        
    }
}