# Grammar for Java that we will be using (also used as checklist hehe)


#### Note:
Donzo     == `[x]`
Not done  == `[ ]`
Also checklist out of date

### util stuffs that everything uses
```
[x] <qualified_name>  ::= IDENTIFIER {"." IDENTIFIER}
[x] <annotation>      ::= "@" <qualified_name> ["(" <skip_parens> ")"]
[x] <modifier>        ::= "public" | "private" | "protected" | "abstract" | "static" | "final" | "strictfp"
[x] <modifiers>       ::= { <modifiers> }
[x] <voidable_type>   ::= "void" | <ref_type>
[x] <ref_type>        ::= <qualified_name> [ "<" <type_arg_lst> ">" ] { "[]" }
[x] <type_arg_list>   ::= <type_arg> { "," <type_arg> }
[x] <type_arg>        ::= <ref_type> | "?" [ ( "extends" | "super" ) <ref_type> ]
[x] <type_param_list> ::= ["<" <type_param> { "," <type_param> } ">"]
[x] <type_param>      ::= IDENTIFIER [ "extends" <ref_type> { "&" <ref_type> } ]
```

### declare pacakge, import files, then declare types
```
[ ] <java_file> ::= <package_decl> <import> {<type_decl>}
```

### package, import
```
[x] <package_decl>  ::= [ "package" <qualified_name> ";" ]
[x] <import>        ::= { "import" [ "static" ] <qualified_name> [ ".*" ] ";" }
```


#### Note: 
- Type params is the parameter, e.g. `public <T> T at(int i) {...}`
- Type args is the argument, e.g. `ArrayList<int> lst;`

### type: class, enum, interface, annotation
```
[x] <type_decl>       ::= {<annotation>} <modifiers> ( <enum_decl> | <class_decl> | <interface_decl> | 
                      <annotation_decl> )
[ ] <enum_decl>       ::= "enum" IDENTIFIER [ "implements" <ref_type> { "," <ref_type> } ] "{" <enum_body> "}"
[ ] <class_decl>      ::= "class" IDENTIFIER [ "extends" <ref_type> ] 
                      [ "implements" <ref_type> { "," <ref_type> } ] <class_body>
[ ] <interface_decl>  ::= "interface" IDENTIFIER [ "extends" <ref_type> { "," <ref_type> } ] 
                      "{" <skip_interface_body> "}"
[ ] <annotation_decl> ::= "@interface" IDENTIFIER "{" <skip_annotation_body> "}"
```

### Body for a class: properties, functions
```
[ ] <class_body>      ::= "{" {<member_decl>} "}"
[ ] <member_decl>     ::= {<annotation>} <modifiers> ( <method_decl> | <property_decl> | 
                      <enum|class|interface|annotation_decl> )
[ ] <method_decl>     ::= [<type_params>] <voidable_type> IDENTIFIER "(" <arg_list> ")" 
                      ["throws" <ref_type> {"," <ref_type>}] "{" <skip_body> "}"
[ ] <property_decl>   ::= <ref_type> IDENTIFIER [ "=" <skip_expr> ] {"," IDENTIFIER [ "=" <skip_expr>]} ";"
```

### Body for enum
```
[ ] <enum_body> ::= {<enum_val>} [ ";" [ <class_body> ] ]
[ ] <enum_val>  ::= IDENTIFIER ["(" <skip_paren> ")"] ["{" <skip_brace> "}"]
```

### Note
All `<skip_something>` nonterminals are skipped via some sort of stack counting
