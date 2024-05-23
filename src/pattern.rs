/*
TO BE MORE EFFICIENT ASSUME THAT PATTERN IS IN ASCENDING ORDER
*/

// 86B overhead with VPN: 1428+86=1514B -> Or else fragment
pub const PATTERN: [usize; 3] = [467, 933, 1400];

// Largest size possible in pattern
const MTU: usize = 1500;
pub const CHAFF: [u8; MTU] = [0; MTU];
const WRAP_AND_WIREGUARD_OVERHAD: f64 = 100.0;

pub fn get_sorted_indices() -> Vec<usize> {
    // Gets sorted indices needed to match incoming packets and the corresponding queue index to choose
    let mut indices: Vec<usize> = (0..PATTERN.len()).collect();
    // Sort the indices based on the corresponding values in the data vector
    indices.sort_by_key(|&i| &PATTERN[i]);
    indices
}

pub fn get_push_state_vector() -> Vec<(usize,usize)> {
    // Store in a vector the ranges of each state [state_start_index,next_state_start[
    // Fancy encoding so no need to use hash maps and reduce overhead compared to accessing lists. 
    // Since patterns are relatively small and in increasing order it should be ok.
    // Make the vector as long as the pattern for easier processing after
    // e.g PATTERN=[100,200,300,300,300,500] gives [(0,1),(1,2),(2,5),(2,5),(2,5),(5,6)]
    // First number in tuple is next queue to push to, second number is the index at which the next state starts
    let mut state = Vec::new();
    let mut count = 0;

    let mut previous_state = 0;
    for i in 0..PATTERN.len() {
        if i < PATTERN.len()-1 && PATTERN[i] == PATTERN[i+1] {
            count += 1
        } else {
            for _ in 0..count+1 {
                state.push((previous_state,i+1));
            }
            previous_state = i+1;
            count = 0;
        }
    }
    state
}

pub fn get_average_pattern_length() -> f64 {
    let mut total = 0.0;
    for p in PATTERN {
        total += p as f64;
    }
    total / PATTERN.len() as f64 + WRAP_AND_WIREGUARD_OVERHAD
}