# Grammar for Java that we will be using

#### Note:
Donzo     == `[X]`
Not done  == `[ ]`

### util stuffs that everything uses
```
[ ] <qualified_name>  ::= IDENTIFIER {"." IDENTIFIER}
[ ] <annotation>      ::= "@" <qualified_name> ["(" <skip_parens> ")"]
[ ] <modifier>        ::= "public" | "private" | "protected" | "abstract" | "static" | "final" | "strictfp"
[ ] <modifiers>       ::= { <modifiers> }
[ ] <voidable_type>   ::= "void" | <ref_type>
[ ] <ref_type>        ::= <qualified_name> [ "<" <type_arg_lst> ">" ] { "[]" }
[ ] <type_arg_list>   ::= <type_arg> { "," <type_arg> }
[ ] <type_arg>        ::= <ref_type> | "?" [ ( "extends" | "super" ) <ref_type> ]
[ ] <type_params>     ::= "<" <type_param> { "," <type_param> } ">"
[ ] <type_param>      ::= IDENTIFIER [ "extends" <ref_type> { "&" <ref_type> } ]
```

### declare pacakge, import files, then declare types
```
[ ] <java_file> ::= <package_decl> <import> {<type_decl>}
```

### package, import
```
[ ] <package_decl>  ::= [ "package" <qualified_name> ";" ]
[ ] <import>        ::= { "import" [ "static" ] <qualified_name> [ ".*" ] ";" }
```


#### Note: 
- Type params is the parameter, e.g. `public <T> T at(int i) {...}`
- Type args is the argument, e.g. `ArrayList<int> lst;`

### type: class, enum, interface, annotation
```
[ ] <type_decl>       ::= {<annotation>} <modifiers> ( <enum_decl> | <class_decl> | <interface_decl> | 
                      <annotation_decl> )
[ ] <enum_decl>       ::= "enum" IDENTIFIER [ "implements" <ref_type> { "," <ref_type> } ] "{" <enum_body> "}"
[ ] <class_decl>      ::= "class" IDENTIFIER [ "extends" <ref_type> ] 
                      [ "implements" <ref_type> { "," <ref_type> } ] "{" <class_body> "}"
[ ] <interface_decl>  ::= "interface" IDENTIFIER [ "extends" <ref_type> { "," <ref_type> } ] 
                      "{" <skip_interface_body> "}"
[ ] <annotation_decl> ::= "@interface" IDENTIFIER "{" <skip_annotation_body> "}"
```

### Body for a class: properties, functions
```
[ ] <class_body>      ::= { <member_decl> }
[ ] <member_decl>     ::= {<annotation>} <modifiers> ( <method_decl> | <property_decl> | <type_decl> )
[ ] <method_decl>     ::= [<type_params>] [<voidable_type>] IDENTIFIER "(" <arg_list> ")" 
                      ["throws" <ref_type> {"," <ref_type>}] "{" <skip_body> "}"
[ ] <property_decl>   ::= <ref_type> IDENTIFIER [ "=" <skip_expr> ] ";"

```

### Body for enum
```
[ ] <enum_body> ::= {<enum_val>} [ ";" [ <class_body> ] ]
[ ] <enum_val>  ::= IDENTIFIER ["(" <skip_paren> ")"] ["{" <skip_brace> "}"]
```

### Note
All `<skip_something>` nonterminals are skipped via some sort of stack counting
