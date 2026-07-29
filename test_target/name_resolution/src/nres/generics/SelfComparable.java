package nres.generics;

// Self-referencing generic bound: T's own bound refers back to T.
public interface SelfComparable<T extends SelfComparable<T>> {
    int compareTo(T other);
}
