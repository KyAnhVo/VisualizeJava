package library.core;

import java.util.List;
import java.util.Optional;

public interface Repository<T extends Identifiable<ID>, ID> {
    void save(T item);

    Optional<T> findById(ID id);

    List<T> findAll();

    void deleteById(ID id);
}
