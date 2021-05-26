module 0x7::Counter {
    use 0x1::Signer;
    use 0x1::Vault;

    struct Counter has store, drop { i: u64 }

    public fun ADMIN_ADDRESS(): address {
        @0x7
    }

    const AUTH_FAILED: u64 = 1;

    public fun publish(admin: &signer) {
        assert(Signer::address_of(admin) == ADMIN_ADDRESS(), AUTH_FAILED);
        Vault::new<Counter>(admin, Counter { i: 0 });
    }

    public fun delegate_read_cap(admin: &signer, delegatee: &signer) {
        if (!Vault::is_delegation_enabled<Counter>(admin)) {
            Vault::enable_delegation<Counter>(admin)
        };
        let delegate_cap = Vault::acquire_delegate_cap<Counter>(admin);
        Vault::delegate<Counter>(&delegate_cap, delegatee, Vault::read_cap_type());
    }

    public fun get_counter(account: &signer): u64 {
        let read_cap = Vault::acquire_read_cap<Counter>(account);
        let read_accessor = Vault::read_accessor<Counter>(&read_cap);
        let ref_counter = Vault::borrow<Counter>(&read_accessor);
        let counter_i = ref_counter.i;
        Vault::release_read_accessor<Counter>(read_accessor);
        counter_i
    }

    public fun increment(account: &signer) {
        let modify_cap = Vault::acquire_modify_cap<Counter>(account);
        let modify_accessor = Vault::modify_accessor<Counter>(&modify_cap);
        let mut_ref_counter = Vault::borrow_mut<Counter>(&mut modify_accessor);
        mut_ref_counter.i = mut_ref_counter.i + 1;
        Vault::release_modify_accessor<Counter>(modify_accessor);
    }

    /*
    This Counter module stores the counter value in a Vault.
    Suppose that this module has the following requirements (R1)-(R3):
    (R1) Only the admin can publish Counter.
        (Suppose that `Counter` is exclusively for the admin)
    (R2) Only the admin can increase the counter value.
    (R3) No one can decrease the counter value
        (i.e., the counter value is never decreased).

    Let's assume that the Vault library is correct. Then, let's see what else we need to
    ensure each of the requirements (R1)-(R3).



    (R1) Only the admin can publish Counter.

    For (R1), we need to ensure that:
        (1) `Counter::publish` aborts if the signer argument is not the admin address,
        (2) no function other than `publish` publishes Vault<Counter>, and
        (3) no function returns an instance of Counter struct.
    We want (3) as well because if a Counter struct is publically returned,
    `Vault<Counter>` can be published from outside of this module.



    (R2) Only the admin can increase the counter value.
        (== You're not allowed to increase the counter value unless you're the admin.)

    It's not easy to directly ensure (R2). Instead, by the help of Vault, we can ensure
    a stronger property:
    (R2') Only the admin can "modify" the counter value.
        (== You're not allowed to modify the counter value unless you're the admin.)

    Note that (R2') implies (R2) because if you can't modify it, you can't increase it either.

    Ensuring (R2') is not completely free. For (R2'), we still need to ensure that either:
        (1) the delegation is not enabled for the vault, OR
        (2) none of its delegates has a modify-cap.
    Formally,
        invariant !Vault::is_delegation_enabled<Counter>(ADMIN_ADDRESS()) ||
            (forall addr in global<VaultDelegates<Counter>>(ADMIN_ADDRESS()):
                forall cap in global<VaultDelegate<Counter>>(addr).granted_caps:
                    cap != Vault::modify_cap_type()
            )



    (R3) No one can decrease the counter value.
        (== The counter value is never decreased.)

    This could be specified as a struct "update" invaraint (by the way, does Prover support this?):
        struct Counter has store, drop { i: u64 }
        spec Counter {
            invariant update old(i) <= i;
        }
    */
}
