use super::super::{parser::Parser, token::Token::*};
use crate::types::*;

impl<'a> Parser<'a> {
    /// ```
    /// <class_decl> ::= "class" IDENTIFIER <type_param_list>
    ///     ["extends" <ref_type>]
    ///     ["implements" <ref_type> {"," <ref_type>}]
    ///     ["permits" <ref_type> {"," <ref_type>}]
    ///     <class_body>
    /// ```
    pub(crate) fn class_decl(&mut self, prefix: QualifiedName<'a>) -> ParseResult<'a, Type<'a>> {
        let ctx = ("class_decl", self.peek_next_token().addr);
        // "class"
        if self.get_next_token().token != Keyword("class") {
            return Err(ParseErrType::UnexpectedToken {
                expected: "class",
                got: vec![self.get_current_token().token],
            }
            .to_stack_parse_err(self.get_current_token().addr, ctx));
        }

        // IDENTIFIER
        let mut name = QualifiedName(prefix.0.clone());
        name.0.push(match self.get_next_token().token {
            Identifier(s) => s,
            token => {
                return Err(ParseErrType::UnexpectedToken {
                    expected: "IDENTIFIER",
                    got: vec![token],
                }
                .to_stack_parse_err(self.get_current_token().addr, ctx));
            }
        });

        // <type_param> (unimportant for now)
        self.type_param_list().push_context(ctx)?;

        // ["extends" <ref_type>]
        let inherits_from: Option<RefType<'a>> =
            if self.peek_next_token().token == Keyword("extends") {
                self.get_next_token();
                Some(self.ref_type().push_context(ctx)?)
            } else {
                None
            };

        // ["implements" <ref_type> {"," <ref_type>}]
        let implement_interfaces: Vec<RefType<'a>> =
            if self.peek_next_token().token == Keyword("implements") {
                self.get_next_token();
                let mut vector = vec![self.ref_type().push_context(ctx)?];
                while self.peek_next_token().token == Comma {
                    self.get_next_token();
                    vector.push(self.ref_type().push_context(ctx)?)
                }
                vector
            } else {
                vec![]
            };

        // ["permits" <ref_type> {"," <ref_type>}]
        if self.peek_next_token().token == Keyword("permits") {
            self.get_next_token();
            self.ref_type().push_context(ctx)?;
            while self.peek_next_token().token == Comma {
                self.get_next_token();
                self.ref_type().push_context(ctx)?;
            }
        }

        let type_kind = TypeKind::Class {
            inherit_class: inherits_from,
            implement_interfaces,
        };

        // <class_body>
        consume_token!(self, ctx, LBrace, "LBrace");
        let body = self
            .members(name.clone(), name.0.last().unwrap())
            .push_context(ctx)?;
        consume_token!(self, ctx, RBrace, "RBrace");

        // use default modifiers and annotation
        let typeclass = Type {
            name,
            modifiers: Modifiers {
                modifiers: vec![],
                access_modifier: AccessModifier::Default,
            },
            type_kind,
            body,
            annotation: vec![],
        };

        Ok(typeclass)
    }
}

//-----------------------------------------------------------------
//--------------------------- UNIT TEST ---------------------------
//-----------------------------------------------------------------

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_class_decl() {
        let mut parser = Parser::new(
            "class MyClass<T extends Comparable<T>> extends MyParentClass<T> implements Printable, GetTAble {
                @NotNull @JsonIgnore
                java.util.HashMap<String, Integer> a,
                    b = new HashMap<String, Integer>(),
                    c;

                @Nullable
                float fa = 1.0f, fb = math.PI / 6, fc = fb * 7;

                @NotNull
                float fe, fh = fa;

                @NotNull
                float fg;

                @Getter
                public Integer fromA(@NotNull String key) {
                    return this.a.get(key);
                }

                public MyClass() {}
                public <T1, T2> MyClass(T1 foo, T2 bar) { ... }
                

                public <T> T getT(String key, java.util.HashMap<Integer, T> hashmap) {
                    return hashmap.get(this.a.get(key));
                }

                abstract public int joinAbc();

                @annotation1(val1, val2)
                public static enum MyEnum { MyEnum1, MyEnum2(v1, v2){}; public String printEnum() {} }
                public static interface MyInterface { public String hash(); public int base64(); }

            }",
        )
        .unwrap();
        let res: Type = parser.class_decl(QualifiedName(vec![])).unwrap();
        println!("res:\n {:#?}", res);
        assert_eq!(res.name, QualifiedName(vec!["MyClass"]));
    }
}
