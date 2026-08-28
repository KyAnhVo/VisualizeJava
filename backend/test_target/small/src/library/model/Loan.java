package library.model;

public class Loan implements library.core.Identifiable<String> {
    private final String loanId;
    private final Book book;
    private final Member member;
    private final int dueDay;
    private LoanStatus status;

    public Loan(String loanId, Book book, Member member, int dueDay) {
        this.loanId = loanId;
        this.book = book;
        this.member = member;
        this.dueDay = dueDay;
        this.status = LoanStatus.ACTIVE;
    }

    @Override
    public String getId() {
        return loanId;
    }

    public Book getBook() {
        return book;
    }

    public Member getMember() {
        return member;
    }

    public int getDueDay() {
        return dueDay;
    }

    public LoanStatus getStatus() {
        return status;
    }

    public void setStatus(LoanStatus status) {
        this.status = status;
    }

    public boolean isOverdue(int currentDay) {
        return status == LoanStatus.ACTIVE && currentDay > dueDay;
    }
}
