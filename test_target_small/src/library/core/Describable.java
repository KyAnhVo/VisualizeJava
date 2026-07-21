package library.core;

public interface Describable {
    String describe();

    default String describeWithPrefix(String prefix) {
        return prefix + ": " + describe();
    }
}
