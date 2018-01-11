use syntex_syntax::parse::parser::Parser;
use syntex_syntax::parse::token::{Token, DelimToken};
use syntex_syntax::parse::common::SeqSep;
use syntex_syntax::symbol::keywords as keyword;
use syntex_syntax::diagnostics::plugin::DiagnosticBuilder;
use syntex_syntax::ast::{MetaItem, MetaItemKind, LitKind};

use super::table::{Table, Col, TableKind};
#[allow(unused_imports)]
use super::{warn, error};

macro_rules! err {
    ($parser:expr, $($args:tt),*) => {{
        return Err($parser.sess.span_diagnostic.struct_span_err($parser.span, &format!($($args),*)));
    }}
}

/*
 * table! {
 *     #[some_attribute]
 *     pub domain_name/table_name {
 *         observing: SegCol<::watchers::MyTable::RowId>,
 *         position: VecCol<MyCoordinate>,
 *         color: SegCol<RgbHexColor>,
 *         is_active: BoolCol,
 *     }
 *
 *     impl {
 *         something_or_other;
 *     }
 * }
 *
 * */
pub fn parse_table<'a>(parser: &mut Parser<'a>) -> Result<Table, DiagnosticBuilder<'a>> {
    let commas = SeqSep::trailing_allowed(Token::Comma);
    let mut table = Table::new();

    fn meta_arg(attr: &MetaItem) -> String {
        if let MetaItemKind::NameValue(ref lit) = attr.node {
            if let LitKind::Str(ref sym, _) = lit.node {
                return format!("{}", sym.as_str());
            }
        }
        panic!("Attributes should be of the form #[name = \"value\"]")
    }

    // [#[attr]] [pub] DOMAIN_NAME::table_name { ... }
    for attr in parser.parse_outer_attributes()?.into_iter() {
        match format!("{}", attr.value.name).as_str() {
            "kind" => table.set_kind(match meta_arg(&attr.value).as_str() {
                "append" => TableKind::Append,
                "public" => TableKind::Public,
                "bag" => TableKind::Bag,
                e => err!(parser, "Unknown kind {:?}", e),
            }),
            "rowid" => table.row_id = meta_arg(&attr.value),
            _ => {
                // other attrs go on the module
                table.module_attrs.push(attr);
            },
        }
    }
    if table.kind.is_none() {
        err!(parser, "Table kind not set");
    }
    table.is_pub = parser.eat_keyword(keyword::Pub);
    //parser.expect(&Token::Mod)?;
    table.domain = parser.parse_ident()?.to_string();
    parser.expect(&Token::ModSep)?;
    table.name = parser.parse_ident()?.to_string();


    // Load structure
    let structure_block = parser.parse_token_tree()?;
    table.cols = {
        let mut parser = Parser::new(parser.sess, vec![structure_block], None, true);
        parser.expect(&Token::OpenDelim(DelimToken::Brace))?;
        parser.parse_seq_to_end(&Token::CloseDelim(DelimToken::Brace), commas, |parser| {
            // #[attrs] column_name: [ElementType; ColumnType<ElementType>],
            let attrs = parser.parse_outer_attributes()?;
            let name = parser.parse_ident()?;
            parser.expect(&Token::Colon)?;
            parser.expect(&Token::OpenDelim(DelimToken::Bracket))?;
            let element = parser.parse_ty()?;
            parser.expect(&Token::Semi)?;
            let colty = parser.parse_ty()?;
            parser.expect(&Token::CloseDelim(DelimToken::Bracket))?;
            Ok(Col {
                attrs,
                name,
                element,
                colty,
                indexed: false,
            })
        })?
    };

    // What tokens remain?
    for t in parser.tts.iter() {
        err!(parser, "Unexpected tokens at end of `table!`: {:?}", t);
    }
    Ok(table)
}
