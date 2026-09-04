package library.service;

import java.util.ArrayList;
import java.util.List;

import static library.core.Constants.MAX_LOANS;

import library.core.Constants;
import library.model.*;
import library.repository.BookRepository;
import library.repository.MemberRepository;

public class LibraryService {
    private final BookRepository bookRepository = new BookRepository();
    private final MemberRepository memberRepository = new MemberRepository();
    private final List<Loan> loans = new ArrayList<>();
    private int loanCounter = 0;
    private int currentDay = 0;

    public void addBook(Book book) {
        bookRepository.save(book);
    }

    public void addMember(Member member) {
        memberRepository.save(member);
    }

    public BookRepository getBookRepository() {
        return bookRepository;
    }

    public MemberRepository getMemberRepository() {
        return memberRepository;
    }

    public void advanceDay(int days) {
        currentDay += days;
    }

    public Loan checkout(String isbn, String memberId) {
        Book book = bookRepository.findById(isbn)
                .orElseThrow(() -> new IllegalStateException("Unknown book: " + isbn));
        Member member = memberRepository.findByIdOrThrow(memberId);

        if (!book.isAvailable()) {
            throw new IllegalStateException("Book already checked out: " + book.getTitle());
        }
        if (member.getActiveLoanCount() >= MAX_LOANS) {
            throw new IllegalStateException("Member has reached the loan limit: " + member.getName());
        }

        book.setAvailable(false);
        member.incrementLoanCount();
        loanCounter++;
        Loan loan = new Loan("L" + loanCounter, book, member, currentDay + Constants.LOAN_PERIOD_DAYS);
        loans.add(loan);
        return loan;
    }

    public void returnLoan(String loanId) {
        for (Loan loan : loans) {
            if (loan.getId().equals(loanId) && loan.getStatus() != LoanStatus.RETURNED) {
                loan.setStatus(LoanStatus.RETURNED);
                loan.getBook().setAvailable(true);
                loan.getMember().decrementLoanCount();
                return;
            }
        }
        throw new IllegalStateException("Unknown or already-returned loan: " + loanId);
    }

    public List<Loan> findOverdueLoans() {
        List<Loan> overdue = new ArrayList<>();
        for (Loan loan : loans) {
            if (loan.isOverdue(currentDay)) {
                loan.setStatus(LoanStatus.OVERDUE);
                overdue.add(loan);
            }
        }
        return overdue;
    }

    public List<Loan> getLoans() {
        return loans;
    }
}
