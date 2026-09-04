package nres.generics;

import java.util.HashMap;
import java.util.Map;

// Abstract class carrying the same mutually referencing bounds, and
// implementing a generic interface with its own type params instantiated
// from this class's type params (not concrete types).
public abstract class Repo<T extends Identifiable2<ID>, ID> implements Store<T, ID> {
    protected final Map<ID, T> storage = new HashMap<>();

    @Override
    public void save(T item) {
        storage.put(item.getId(), item);
    }

    @Override
    public T findById(ID id) {
        return storage.get(id);
    }

    public int count() {
        return storage.size();
    }
}
