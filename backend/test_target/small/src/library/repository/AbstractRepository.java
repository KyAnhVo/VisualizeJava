package library.repository;

import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.Optional;

import library.core.Identifiable;
import library.core.Repository;

public abstract class AbstractRepository<T extends Identifiable<ID>, ID> implements Repository<T, ID> {
    protected final Map<ID, T> storage = new HashMap<>();

    @Override
    public void save(T item) {
        storage.put(item.getId(), item);
    }

    @Override
    public Optional<T> findById(ID id) {
        return Optional.ofNullable(storage.get(id));
    }

    @Override
    public List<T> findAll() {
        return new ArrayList<>(storage.values());
    }

    @Override
    public void deleteById(ID id) {
        storage.remove(id);
    }

    public int count() {
        return storage.size();
    }
}
