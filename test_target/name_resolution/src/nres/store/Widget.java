package nres.store;

import nres.generics.Identifiable2;
import nres.generics.SelfComparable;

public class Widget implements Identifiable2<String>, SelfComparable<Widget> {
    private final String id;
    private final int weight;

    public Widget(String id, int weight) {
        this.id = id;
        this.weight = weight;
    }

    @Override
    public String getId() {
        return id;
    }

    @Override
    public int compareTo(Widget other) {
        return this.weight - other.weight;
    }
}
