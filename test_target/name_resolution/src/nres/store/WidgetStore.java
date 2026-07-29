package nres.store;

import nres.generics.Repo;

// Two-hop inheritance: WidgetStore -> Repo -> (implements) Store,
// with Repo's class-level type params instantiated to concrete types here.
public class WidgetStore extends Repo<Widget, String> {
    public boolean isEmpty() {
        return count() == 0;
    }
}
