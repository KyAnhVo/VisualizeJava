package nres.generics;

// Mutually referencing bounds: T's bound refers to ID, which is declared
// after T in the same type-param list.
public interface Store<T extends Identifiable2<ID>, ID> {
    void save(T item);

    T findById(ID id);
}
