package nres.status;

import nres.common.Describable;

// Enum implementing an interface.
public enum Priority implements Describable {
    LOW("Low priority"),
    MEDIUM("Medium priority"),
    HIGH("High priority");

    private final String label;

    Priority(String label) {
        this.label = label;
    }

    @Override
    public String describe() {
        return label;
    }
}
