package library.model;

import library.annotations.Field;
import library.core.Describable;
import library.core.Identifiable;

public class Book implements Identifiable<String>, Describable, Comparable<Book> {
    @Field(label = "ISBN")
    private final String isbn;

    @Field(label = "Title")
    private final String title;

    @Field(label = "Author")
    private final String author;

    @Field(label = "Genre", required = false)
    private final Genre genre;

    private boolean available;

    private Book(Builder builder) {
        this.isbn = builder.isbn;
        this.title = builder.title;
        this.author = builder.author;
        this.genre = builder.genre;
        this.available = true;
    }

    @Override
    public String getId() {
        return isbn;
    }

    public String getTitle() {
        return title;
    }

    public String getAuthor() {
        return author;
    }

    public Genre getGenre() {
        return genre;
    }

    public boolean isAvailable() {
        return available;
    }

    public void setAvailable(boolean available) {
        this.available = available;
    }

    @Override
    public String describe() {
        return title + " by " + author + " (" + genre + ")";
    }

    @Override
    public int compareTo(Book other) {
        return this.title.compareTo(other.title);
    }

    public static Builder builder() {
        return new Builder();
    }

    public static class Builder {
        private String isbn;
        private String title;
        private String author;
        private Genre genre;

        public Builder isbn(String isbn) {
            this.isbn = isbn;
            return this;
        }

        public Builder title(String title) {
            this.title = title;
            return this;
        }

        public Builder author(String author) {
            this.author = author;
            return this;
        }

        public Builder genre(Genre genre) {
            this.genre = genre;
            return this;
        }

        public Book build() {
            return new Book(this);
        }
    }
}
