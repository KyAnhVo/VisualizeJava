package library.model;

import library.annotations.Field;
import library.core.Describable;
import library.core.Identifiable;

public class Member implements Identifiable<String>, Describable {
    @Field(label = "Member ID")
    private final String memberId;

    @Field(label = "Name")
    private final String name;

    private int activeLoanCount;

    public Member(String memberId, String name) {
        this.memberId = memberId;
        this.name = name;
        this.activeLoanCount = 0;
    }

    @Override
    public String getId() {
        return memberId;
    }

    public String getName() {
        return name;
    }

    public int getActiveLoanCount() {
        return activeLoanCount;
    }

    public void incrementLoanCount() {
        activeLoanCount++;
    }

    public void decrementLoanCount() {
        if (activeLoanCount > 0) {
            activeLoanCount--;
        }
    }

    @Override
    public String describe() {
        return name + " (" + memberId + ")";
    }
}
