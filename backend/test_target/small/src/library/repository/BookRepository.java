package library.repository;

import java.util.List;
import java.util.stream.Collectors;

import library.model.Book;
import library.model.Genre;

public class BookRepository extends AbstractRepository<Book, String> {
    public List<Book> findByGenre(Genre genre) {
        return findAll().stream()
                .filter(book -> book.getGenre() == genre)
                .collect(Collectors.toList());
    }

    public List<Book> findAvailable() {
        return findAll().stream()
                .filter(Book::isAvailable)
                .collect(Collectors.toList());
    }
}
