package library.repository;

import library.model.Member;

public class MemberRepository extends AbstractRepository<Member, String> {
    public Member findByIdOrThrow(String id) {
        return findById(id).orElseThrow(() ->
                new IllegalArgumentException("No member with id " + id));
    }
}
