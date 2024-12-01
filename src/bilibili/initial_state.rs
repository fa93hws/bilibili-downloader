use std::fmt::Debug;

use swc_common::source_map::SmallPos;
use swc_common::sync::Lrc;
use swc_common::{Span, Spanned};
use swc_common::{
    errors::{ColorConfig, Handler},
    FileName, SourceMap,
};
use swc_ecma_ast::{AssignTarget, Expr, MemberProp, Module, SimpleAssignTarget};
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax};
use swc_ecmascript::visit::{Visit, VisitWith};
use anyhow::{anyhow, Result};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
struct VideoDataSpec {
    pub cid: i64,
    pub bvid: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct InitialStateSpec {
    pub video_data: VideoDataSpec,
}

pub struct InitialState {
    pub cid: i64,
    pub bvid: String,
}

fn parse_js(content: &str) -> Result<Module> {
    let cm: Lrc<SourceMap> = Default::default();
    let handler =
        Handler::with_tty_emitter(ColorConfig::Auto, true, false,
        Some(cm.clone()));
    let fm = cm.new_source_file(
            FileName::Custom("not_matter.js".into()).into(),
            content.into(),
    );
    let lexer = Lexer::new(
        Syntax::Es(Default::default()),
        Default::default(),
        StringInput::from(&*fm),
        None,
    );
    let mut parser = Parser::new_from(lexer);
    for e in parser.take_errors() {
        e.into_diagnostic(&handler).emit();
    }

    Ok(parser
        .parse_module()
        .map_err(|e| {
            // Unrecoverable fatal error occurred
            e.into_diagnostic(&handler).emit()
        })
        .expect("failed to parser module"))
}

struct InitialStateVisitor {
    pub object_span: Option<Span>,
}

impl Visit for InitialStateVisitor {
    fn visit_assign_expr(&mut self, node: &swc_ecma_ast::AssignExpr) {
        let AssignTarget::Simple(simple_target) = &node.left else {
            return;
        };
        let SimpleAssignTarget::Member(member_target) = simple_target else {
            return;
        };
        let Expr::Ident(ident) = *member_target.obj.clone() else {
            return;
        };
        if ident.sym.to_string() != "window" {
            return;
        }
        let MemberProp::Ident(ident) = &member_target.prop else {
            return;
        };
        if ident.sym.to_string() == "__INITIAL_STATE__" {
            self.object_span = Some(node.right.span());
        }
    }
}

fn initial_state_visitor() -> InitialStateVisitor {
    InitialStateVisitor {
        object_span: None,
    }
}

fn try_extract_from_code(ast: &Module, content: &str) -> Option<String> {
    let mut visitor = initial_state_visitor();
    ast.visit_with(&mut visitor);
    if let Some(span) = visitor.object_span {
        let low = span.lo.to_usize() - 1;
        // trim ending ;
        let hi = span.hi.to_usize() - 1;
        return Some(content[low..hi].to_string());
    }
    None
}

pub fn extract_initial_state(document: &Html) -> Result<InitialState> {
    let script_selector = Selector::parse(r#"script:not([type*=json])"#).expect("failed to parse selector");
    for script_element in document.select(&script_selector) {
        let script_content = script_element.text().collect::<Vec<_>>().join("");
        let ast = parse_js(&script_content)?;
        if let Some(json_string) = try_extract_from_code(&ast, &script_content) {
            let initial_state = serde_json::from_str::<InitialStateSpec>(&json_string)
                .expect("failed to parse initial state");
            return Ok(InitialState {
                bvid: initial_state.video_data.bvid,
                cid: initial_state.video_data.cid,
            });
        };
    }
    Err(anyhow!("failed to find __INITIAL_STATE__ assignment"))
}
