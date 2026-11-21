// Checks for the math kernels, run by Goose alone (`just test-goose`).
// A failed assert prints its line and makes the run exit with status 1.
import kernels from "../kernels";

fn main() {
    // FNV-1a published vectors: the empty string is the offset basis.
    assert(kernels::fnv1a("") == 14695981039346656037);
    assert(kernels::fnv1a("a") == 12638187200555641996);

    assert(kernels::count_primes(1) == 0);
    assert(kernels::count_primes(2) == 1);
    assert(kernels::count_primes(100) == 25);
    assert(kernels::count_primes(1000000) == 78498);

    var y: f64[>..] = [1.0, 2.0, 3.0];
    var x: f64[>..] = [10.0, 20.0, 30.0];
    kernels::saxpy(y, x, 0.5);
    assert(y[0] == 6.0 && y[1] == 12.0 && y[2] == 18.0);

    var xs: f64[>..] = [3.0, -1.5, 4.5];
    let s = kernels::summary(xs);
    assert(s.count == 3 && s.min == -1.5 && s.max == 4.5 && s.mean == 2.0);

    // The shared generator must match the reference splitmix64 sequence.
    var state: u64 = 0;
    assert(kernels::splitmix64(state) == 16294208416658607535);
    assert(kernels::splitmix64(state) == 7960286522194355700);

    print("kernels: ok");
}
