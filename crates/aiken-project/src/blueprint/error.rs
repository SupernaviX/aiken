use super::{
    definitions::Reference,
    schema::{self, Schema},
};
use aiken_lang::ast::Span;
use miette::{Diagnostic, NamedSource};
use owo_colors::{OwoColorize, Stream::Stdout};
use std::fmt::Debug;

#[derive(Debug, thiserror::Error, Diagnostic)]
pub enum Error {
    #[error("{}", error)]
    #[diagnostic(help("{}", error.help()))]
    #[diagnostic(code("aiken::blueprint::interface"))]
    Schema {
        error: schema::Error,
        #[label("invalid validator's boundary")]
        location: Span,
        #[source_code]
        source_code: NamedSource,
    },

    #[error("Invalid or missing project's blueprint file.")]
    #[diagnostic(code("aiken::blueprint::missing"))]
    #[diagnostic(help(
        "Did you forget to {build} the project?",
        build = "build"
            .if_supports_color(Stdout, |s| s.purple())
            .if_supports_color(Stdout, |s| s.bold())
    ))]
    InvalidOrMissingFile,

    #[error("I didn't find any parameters to apply in the given validator.")]
    #[diagnostic(code("aiken::blueprint::apply::no_parameters"))]
    NoParametersToApply,

    #[error(
        "I couldn't compute the address of the given validator because it's parameterized by {} parameter(s)!",
        n.if_supports_color(Stdout, |s| s.purple())
    )]
    #[diagnostic(code("aiken::blueprint::address::parameterized"))]
    #[diagnostic(help(
        "I can only compute addresses of validators that are fully applied. For example, a {keyword_spend} validator must have exactly {spend_arity} arguments: a datum, a redeemer and a context. If it has more, they need to be provided beforehand and applied directly to the validator.\n\nApplying parameters change the validator's compiled code, and thus the address. This is why I need you to apply parameters first using the {blueprint_apply_command} command.",
        keyword_spend = "spend".if_supports_color(Stdout, |s| s.yellow()),
        spend_arity = "3".if_supports_color(Stdout, |s| s.yellow()),
        blueprint_apply_command = "blueprint apply".if_supports_color(Stdout, |s| s.purple()),
    ))]
    ParameterizedValidator { n: usize },

    #[error("I failed to infer what should be the schema of a given parameter to apply.")]
    #[diagnostic(code("aiken:blueprint::apply::malformed::argument"))]
    #[diagnostic(help(
        "I couldn't figure out the schema corresponding to a term you've given. Here's a possible hint about why I failed: {hint}"
    ))]
    UnableToInferArgumentSchema { hint: String },

    #[error("I couldn't find a definition corresponding to a reference.")]
    #[diagnostic(code("aiken::blueprint::apply::unknown::reference"))]
    #[diagnostic(help(
        "While resolving a schema definition, I stumble upon an unknown reference:\n\n  {reference}\n\nThis is unfortunate, but signals that either the reference is invalid or that the correspond schema definition is missing.",
        reference = reference.as_json_pointer()
    ))]
    UnresolvedSchemaReference { reference: Reference },

    #[error("I caught a parameter application that seems off.")]
    #[diagnostic(code("aiken::blueprint::apply::mismatch"))]
    #[diagnostic(help(
        "When applying parameters to a validator, I control that the shape of the parameter you give me matches what is specified in the blueprint. Unfortunately, schemas didn't match in this case.\n\nI am expecting the following:\n\n{}But I've inferred the following schema from your input:\n\n{}",
        serde_json::to_string_pretty(&expected).unwrap().if_supports_color(Stdout, |s| s.green()),
        serde_json::to_string_pretty(&inferred).unwrap().if_supports_color(Stdout, |s| s.red()),
    ))]
    SchemaMismatch { expected: Schema, inferred: Schema },
}

unsafe impl Send for Error {}

unsafe impl Sync for Error {}
