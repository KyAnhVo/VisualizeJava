package library.app;

import java.util.List;

import library.model.Book;
import library.model.Genre;
import library.model.Loan;
import library.service.LibraryService;
import library.util.LibraryUtils;

public class Main {
    public static void main(String[] args) {
        LibraryService library = new LibraryService();

        library.addBook(Book.builder()
                .isbn("978-0-13-468599-1")
                .title("Effective Java")
                .author("Joshua Bloch")
                .genre(Genre.SCIENCE)
                .build());
        library.addBook(Book.builder()
                .isbn("978-0-596-00712-6")
                .title("Head First Design Patterns")
                .author("Freeman & Robson")
                .genre(Genre.SCIENCE)
                .build());
        library.addBook(Book.builder()
                .isbn("978-0-14-303943-3")
                .title("A Brief History of Time")
                .author("Stephen Hawking")
                .genre(Genre.NON_FICTION)
                .build());

        library.addMember(new library.model.Member("M1", "Ada Lovelace"));
        library.addMember(new library.model.Member("M2", "Alan Turing"));

        System.out.println("=== Catalog ===");
        LibraryUtils.printAll(library.getBookRepository().findAll());

        System.out.println();
        System.out.println("=== Sorted by title ===");
        for (Book book : LibraryUtils.sorted(library.getBookRepository().findAll())) {
            System.out.println(" - " + book.describe());
        }

        System.out.println();
        System.out.println("=== Checking out books ===");
        Loan loan1 = library.checkout("978-0-13-468599-1", "M1");
        Loan loan2 = library.checkout("978-0-596-00712-6", "M2");
        System.out.println("Checked out: " + loan1.getBook().getTitle() + " to " + loan1.getMember().getName());
        System.out.println("Checked out: " + loan2.getBook().getTitle() + " to " + loan2.getMember().getName());

        library.advanceDay(20);

        System.out.println();
        System.out.println("=== Overdue after 20 days ===");
        List<Loan> overdue = library.findOverdueLoans();
        for (Loan loan : overdue) {
            System.out.println(" - " + loan.getBook().getTitle() + " (" + loan.getStatus().getDescription() + ")");
        }

        System.out.println();
        System.out.println("=== Returning a book ===");
        library.returnLoan(loan1.getId());
        System.out.println(loan1.getBook().getTitle() + " is now available: " + loan1.getBook().isAvailable());

        System.out.println();
        System.out.println("=== Available books ===");
        LibraryUtils.printAll(library.getBookRepository().findAvailable());
    }
}
