// Checks for the contest workloads, run by Goose alone (`just test-goose`).
import kernels from "../kernels";
import kernels.workloads;

fn main() {
    assert(kernels::records_checksum(0, 1) == 0);
    assert(kernels::tree_checksum(0, 1) == 0);

    // Same seed, same answer; another seed, another answer.
    let a = kernels::records_checksum(1000, 42);
    assert(a == kernels::records_checksum(1000, 42));
    assert(a != kernels::records_checksum(1000, 43));

    let t = kernels::tree_checksum(1000, 42);
    assert(t == kernels::tree_checksum(1000, 42));
    assert(t != kernels::tree_checksum(1000, 43));

    // One key: one distinct node, found once, height 1.
    assert(kernels::tree_checksum(1, 7) == 1000003 + 1 + 1);

    print("workloads: ok");
}
