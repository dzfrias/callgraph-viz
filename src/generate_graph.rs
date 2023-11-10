use anyhow::Result;
use rustpython_ast::{
    Expr, ExprAttribute, ExprAwait, ExprBinOp, ExprBoolOp, ExprCall, ExprCompare, ExprDict,
    ExprDictComp, ExprFormattedValue, ExprGeneratorExp, ExprIfExp, ExprJoinedStr, ExprLambda,
    ExprList, ExprListComp, ExprNamedExpr, ExprSet, ExprSetComp, ExprSlice, ExprStarred,
    ExprSubscript, ExprTuple, ExprUnaryOp, ExprYield, ExprYieldFrom, Mod, Stmt, StmtAnnAssign,
    StmtAssert, StmtAssign, StmtAsyncFunctionDef, StmtAugAssign, StmtDelete, StmtExpr, StmtFor,
    StmtFunctionDef, StmtReturn, StmtTry, StmtWhile, StmtWith,
};
use rustpython_parser::Mode;
use std::collections::HashMap;

pub fn generate_graph(src: &str, path: &str) -> Result<HashMap<String, Vec<String>>> {
    let Mod::Module(module) = rustpython_parser::parse(src, Mode::Module, path)? else {
        panic!();
    };

    let mut graph = HashMap::new();
    let mut current_name;
    for stmt in module.body {
        current_name = "...".to_owned();
        match stmt {
            Stmt::FunctionDef(StmtFunctionDef { name, body, .. })
            | Stmt::AsyncFunctionDef(StmtAsyncFunctionDef { name, body, .. }) => {
                current_name = name.to_string();
                graph.insert(current_name.clone(), Vec::new());
                for stmt in body {
                    build_graph_from_stmt(stmt, current_name.clone(), &mut graph);
                }
            }
            _ => {
                build_graph_from_stmt(stmt, current_name.clone(), &mut graph);
            }
        }
    }

    Ok(graph)
}

fn build_graph_from_stmt(stmt: Stmt, func_name: String, graph: &mut HashMap<String, Vec<String>>) {
    match stmt {
        Stmt::Expr(StmtExpr { value, .. }) => {
            build_graph(*value, func_name.to_string(), graph);
        }
        Stmt::Return(StmtReturn { value, .. }) => {
            if let Some(value) = value {
                build_graph(*value, func_name.to_string(), graph);
            }
        }
        Stmt::Assert(StmtAssert { test, msg, .. }) => {
            build_graph(*test, func_name.to_string(), graph);
            if let Some(msg) = msg {
                build_graph(*msg, func_name.to_string(), graph);
            }
        }
        Stmt::Try(StmtTry {
            body,
            orelse,
            finalbody,
            ..
        }) => {
            for stmt in body {
                build_graph_from_stmt(stmt, func_name.clone(), graph);
            }
            for stmt in orelse {
                build_graph_from_stmt(stmt, func_name.clone(), graph);
            }
            for stmt in finalbody {
                build_graph_from_stmt(stmt, func_name.clone(), graph);
            }
        }
        Stmt::With(StmtWith { items, body, .. }) => {
            for item in items {
                build_graph(item.context_expr, func_name.clone(), graph);
                if let Some(item) = item.optional_vars {
                    build_graph(*item, func_name.clone(), graph);
                }
            }
            for stmt in body {
                build_graph_from_stmt(stmt, func_name.clone(), graph);
            }
        }
        Stmt::For(StmtFor {
            target, iter, body, ..
        }) => {
            build_graph(*target, func_name.clone(), graph);
            build_graph(*iter, func_name.clone(), graph);
            for stmt in body {
                build_graph_from_stmt(stmt, func_name.clone(), graph);
            }
        }
        Stmt::Assign(StmtAssign { targets, value, .. }) => {
            for target in targets {
                build_graph(target, func_name.clone(), graph);
            }
            build_graph(*value, func_name.clone(), graph);
        }
        Stmt::AnnAssign(StmtAnnAssign { target, value, .. }) => {
            build_graph(*target, func_name.clone(), graph);
            if let Some(value) = value {
                build_graph(*value, func_name.clone(), graph);
            };
        }
        Stmt::Delete(StmtDelete { targets, .. }) => {
            for target in targets {
                build_graph(target, func_name.clone(), graph);
            }
        }
        Stmt::While(StmtWhile { test, body, .. }) => {
            build_graph(*test, func_name.clone(), graph);
            for stmt in body {
                build_graph_from_stmt(stmt, func_name.clone(), graph);
            }
        }
        Stmt::AugAssign(StmtAugAssign { target, value, .. }) => {
            build_graph(*target, func_name.clone(), graph);
            build_graph(*value, func_name.clone(), graph);
        }
        _ => {}
    }
}

fn build_graph(expr: Expr, func_name: String, graph: &mut HashMap<String, Vec<String>>) {
    let mut current = Vec::new();
    get_call_idents(expr, &mut current);
    for ident in current {
        graph
            .entry(func_name.clone())
            .or_default()
            .push(ident.clone());
        graph.entry(ident).or_default();
    }
}

fn get_call_idents(expr: Expr, current: &mut Vec<String>) {
    match expr {
        Expr::Call(ExprCall { func, args, .. }) => {
            match *func {
                Expr::Attribute(ExprAttribute { attr, .. }) => {
                    current.push(attr.to_string());
                }
                Expr::Name(name) => {
                    current.push(name.id.to_string());
                }
                _ => {}
            }
            for arg in args {
                get_call_idents(arg, current);
            }
        }
        Expr::BoolOp(ExprBoolOp { values, .. })
        | Expr::List(ExprList { elts: values, .. })
        | Expr::Set(ExprSet { elts: values, .. })
        | Expr::JoinedStr(ExprJoinedStr { values, .. })
        | Expr::Tuple(ExprTuple { elts: values, .. }) => {
            for value in values {
                get_call_idents(value, current);
            }
        }
        Expr::Dict(ExprDict { keys, values, .. }) => {
            for value in values {
                get_call_idents(value, current);
            }
            for key in keys.into_iter().filter_map(|x| x) {
                get_call_idents(key, current);
            }
        }
        Expr::NamedExpr(ExprNamedExpr {
            target: left,
            value: right,
            ..
        })
        | Expr::BinOp(ExprBinOp { left, right, .. })
        | Expr::DictComp(ExprDictComp {
            key: left,
            value: right,
            ..
        })
        | Expr::Subscript(ExprSubscript {
            value: left,
            slice: right,
            ..
        }) => {
            get_call_idents(*left, current);
            get_call_idents(*right, current);
        }
        Expr::UnaryOp(ExprUnaryOp { operand, .. })
        | Expr::Lambda(ExprLambda { body: operand, .. })
        | Expr::Await(ExprAwait { value: operand, .. })
        | Expr::Yield(ExprYield {
            value: Some(operand),
            ..
        })
        | Expr::YieldFrom(ExprYieldFrom { value: operand, .. })
        | Expr::ListComp(ExprListComp { elt: operand, .. })
        | Expr::SetComp(ExprSetComp { elt: operand, .. })
        | Expr::GeneratorExp(ExprGeneratorExp { elt: operand, .. })
        | Expr::Attribute(ExprAttribute { value: operand, .. })
        | Expr::Starred(ExprStarred { value: operand, .. }) => {
            get_call_idents(*operand, current);
        }
        Expr::IfExp(ExprIfExp {
            test, body, orelse, ..
        }) => {
            get_call_idents(*test, current);
            get_call_idents(*body, current);
            get_call_idents(*orelse, current);
        }
        Expr::Compare(ExprCompare {
            left, comparators, ..
        }) => {
            get_call_idents(*left, current);
            for comparator in comparators {
                get_call_idents(comparator, current);
            }
        }
        Expr::FormattedValue(ExprFormattedValue {
            value, format_spec, ..
        }) => {
            get_call_idents(*value, current);
            if let Some(format_spec) = format_spec {
                get_call_idents(*format_spec, current);
            }
        }
        Expr::Slice(ExprSlice {
            upper, lower, step, ..
        }) => {
            if let Some(upper) = upper {
                get_call_idents(*upper, current);
            }
            if let Some(lower) = lower {
                get_call_idents(*lower, current);
            }
            if let Some(step) = step {
                get_call_idents(*step, current);
            }
        }
        _ => {}
    }
}
