package library.model;

public enum LoanStatus {
    ACTIVE("currently checked out"),
    RETURNED("returned to the library"),
    OVERDUE("past its due date");

    private final String description;

    LoanStatus(String description) {
        this.description = description;
    }

    public String getDescription() {
        return description;
    }
}
