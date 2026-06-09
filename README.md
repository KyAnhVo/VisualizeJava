# JavaToUML
Your java code doesn't have a UML for it yet? Generate one with JavaToUML!

## Description
This project will capture class relationships and these relationships only:
- inheritance (and thus polymorphism)
- interfacing
- composition

## Example

### Example 1
```java
public class A {
  private B objB;
  public C objC;
}
```
Our UML will capture this relationship with `A*--B` and `A*--C` edges.

### Example 2
```java
public class A implements B {...}
public class C extends D {...}
```
Our UML will capture `A<|..B` and `C<|--D`.

### Example 3
```java
public class A{
  private static class B {...}
  public static class C {...}
  protected static class D {...}
}
```
Our UML will capture this like
```java
public class A {...}
public class A.C {...}
protected class A.D {...}
```

### Example 4
```java
public class A {
  public void f1(B objB) {...}
  private void f2(C objC) {...}
}
```
None of `A..B` nor `A..C` will be captured from functional applications.
