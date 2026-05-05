use super::types::SymbolInfo;

pub(super) fn flatten_symbols(
    symbols: &[lsp_types::DocumentSymbol],
    result: &mut Vec<SymbolInfo>,
    _depth: usize,
) {
    for sym in symbols {
        result.push(SymbolInfo {
            name: sym.name.clone(),
            kind: format!("{:?}", sym.kind),
            line: sym.selection_range.start.line,
            col: sym.selection_range.start.character,
        });
        if let Some(children) = &sym.children {
            flatten_symbols(children, result, _depth + 1);
        }
    }
}

pub(super) fn markup_value_to_string(v: lsp_types::MarkedString) -> String {
    match v {
        lsp_types::MarkedString::String(s) => s,
        lsp_types::MarkedString::LanguageString(ls) => ls.value,
    }
}
