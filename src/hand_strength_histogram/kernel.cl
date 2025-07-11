// Assumed constants and types
#define NUM_RANKS 13
#define NUM_SUITS 4
#define NUM_BINS 30

#define GET_RANK(card) (card >> 2)

// Assume constants defined for hand types
#define STRAIGHT_FLUSH 8
#define FOUR_OF_A_KIND 7
#define FULL_HOUSE 6
#define FLUSH 5
#define STRAIGHT 4
#define THREE_OF_A_KIND 3
#define TWO_PAIR 2
#define ONE_PAIR 1
#define HIGH_CARD 0


// 
// RNG
// 

unsigned int xorshift(unsigned int *state) {
    *state ^= *state << 13;
    *state ^= *state >> 17;
    *state ^= *state << 5;
    return *state;
}

// 
// UTILS
// 

// Function to copy from global to private memory
void copy_global_to_private(__private uchar* dst, __global const uchar* src, int count) {
    for (int i = 0; i < count; i++) {
        dst[i] = src[i];
    }
}

// Function to copy within private memory
void copy_private_to_private(__private uchar* dst, __private const uchar* src, int count) {
    for (int i = 0; i < count; i++) {
        dst[i] = src[i];
    }
}

// Keep only the most significant bit
uint keep_highest(uint rank) {
    return 1 << (31 - clz(rank));
}


// 
// DECK
// 

uchar get_suit(uchar card) {
    // ours: s, h, d, c
    // the library we copied the hand eval from: s, c, h ,d
    uchar rank_ours = (card) & 3;
    if (rank_ours == 1) return 2;
    if (rank_ours == 2) return 3;
    if (rank_ours == 3) return 1;

    return rank_ours;
}

// Initializes the deck of cards
void initialize_deck(uchar *deck) {
    for (int i = 0; i < 52; i++) {
        deck[i] = (uchar)i; // Initialize card values from 0 to 51
    }
}

// Removes hand cards from the deck
void remove_hand_cards(uchar *deck, __global const uchar* hand_cards, int num_hand_cards) {
    for (int i = 0; i < num_hand_cards; i++) {
        for (int j = 0; j < 52; j++) {
            if (deck[j] == hand_cards[i]) {
                deck[j] = 255; // Mark the card as removed by setting it to an invalid value
                break;
            }
        }
    }
}

uchar draw_random_card(uchar *deck, unsigned int *seed) {
    unsigned int local_seed = *seed;
    local_seed = xorshift(seed);

    int index;
    do {
        local_seed = xorshift(&local_seed);
        index = local_seed % 52;
    } while (deck[index] == 255);
    uchar card = deck[index];
    deck[index] = 255;
    return card;
}

void draw_community_cards(uchar *deck, uchar *community_cards, int known_cards_count, unsigned int *seed) {
    int count = known_cards_count;
    while (count < 5) {
        uchar card = draw_random_card(deck, seed);
        community_cards[count++] = card;
        // Debug output for each draw
    }
}

// 
// EVALUATION
//

uint rank_hand(uint hand_type, uint card_details) {
    // printf("card_details: %d \n", card_details);
    return (hand_type << 27) | card_details;
}

// Identifying and ranking a straight in a set of card values
uint rank_straight(uint value_set) {
    uint left = value_set & (value_set << 1) & (value_set << 2) & (value_set << 3) & (value_set << 4);
    int idx = clz(left);
    if (idx < 32) {
        uint highest_card_rank = 32 - 4 - idx; // Highest card in the straight
        return (STRAIGHT << 27) | (1 << highest_card_rank); // Encode as a straight
    } else if ((value_set & 0b1000000001111) == 0b1000000001111) { // Check for a wheel (A-2-3-4-5)
        return (STRAIGHT << 27) | (1 << 3); // Ace is treated as low, so "3" represents the 5
    }
    return 0; // Return zero to indicate no straight, zero is a valid return since no hand type uses it
}

// Keeping the N highest bits
uint keep_n(uint rank, uint to_keep) {
    while (popcount(rank) > to_keep) {
        rank &= rank - 1; // Remove the least significant bit
    }
    return rank;
}

int find_flush(const uint *suit_value_sets) {
    for (int i = 0; i < 4; i++) {
        if (popcount(suit_value_sets[i]) >= 5) { // Using pop count to find at least 5 cards of same suit
            return i;
        }
    }
    return -1;
}

// Hand evaluation function - make sure it handles the right address space
int evaluate_hand(__private const uchar* hand, const int hand_size) {
    uchar value_to_count[13] = {0};
    uint count_to_value[5] = {0};
    uint suit_value_sets[4] = {0};
    uint value_set = 0;

    for (int i = 0; i < 7; i++) {
        uchar card = hand[i];
        uchar v = GET_RANK(card);
        uchar s = get_suit(card);
        value_set |= 1 << v;
        value_to_count[v]++;
        suit_value_sets[s] |= 1 << v;
    }

    // Convert value counts to another form for easier processing
    for (int i = 0; i < 13; i++) {
        uchar count = value_to_count[i];
        count_to_value[count] |= 1 << i;
    }    

    // Initialize variables for evaluating hands
    int flush_index = find_flush(suit_value_sets);

    uint rank_result = 0;

    if (flush_index != -1) {
        uint straight_rank = rank_straight(value_set);
        if (straight_rank != 0) { // If a straight is found
            // printf("straight flush %d\n", rank_hand(STRAIGHT_FLUSH, straight_rank));
            return rank_hand(STRAIGHT_FLUSH, straight_rank);
        }
        uint rank = keep_n(suit_value_sets[flush_index], 5);
        // printf("flush %d \n", rank_hand(FLUSH, rank));
        return rank_hand(FLUSH, rank);
    } else if (count_to_value[4] != 0) {
        uint high = keep_highest(value_set ^ count_to_value[4]);
        // printf("four of a kind %d \n", rank_hand(FOUR_OF_A_KIND, count_to_value[4] << 13 | high));
        return rank_hand(FOUR_OF_A_KIND, count_to_value[4] << 13 | high);
    } else if (popcount(count_to_value[3]) == 2) {
        uint set = keep_highest(count_to_value[3]);
        uint pair = count_to_value[3] ^ set;
        // printf("full house 1 %d \n", rank_hand(FULL_HOUSE, set << 13 | pair));
        return rank_hand(FULL_HOUSE, set << 13 | pair);
    } else if (count_to_value[3] != 0 && count_to_value[2] != 0) {
        uint set = count_to_value[3];
        uint pair = keep_highest(count_to_value[2]);
        // printf("full house 2 %d \n", rank_hand(FULL_HOUSE, set << 13 | pair));
        return rank_hand(FULL_HOUSE, set << 13 | pair);
    } else if (rank_straight(value_set) != 0) {
        // printf("straight %d \n", rank_hand(STRAIGHT, rank_straight(value_set)));
        return rank_hand(STRAIGHT, rank_straight(value_set));
    } else if (count_to_value[3] != 0) {
        uint low = keep_n(value_set ^ count_to_value[3], 2);
        // printf("three of a kind %d \n", rank_hand(THREE_OF_A_KIND, count_to_value[3] << 13 | low));
        return rank_hand(THREE_OF_A_KIND, count_to_value[3] << 13 | low);
    } else if (popcount(count_to_value[2]) >= 2) {
        uint pairs = keep_n(count_to_value[2], 2);
        uint low = keep_highest(value_set ^ pairs);
        // printf("two pair %d \n", rank_hand(TWO_PAIR, pairs << 13 | low));
        return rank_hand(TWO_PAIR, pairs << 13 | low);
    } else if (count_to_value[2] != 0) {
        uint pair = count_to_value[2];
        uint low = keep_n(value_set ^ pair, 3);
        // printf("one pair %d \n", rank_hand(ONE_PAIR, pair << 13 | low));
        return rank_hand(ONE_PAIR, pair << 13 | low);
    } else {
        // printf("high card %d \n", rank_hand(HIGH_CARD, keep_n(value_set, 5)));
        return rank_hand(HIGH_CARD, keep_n(value_set, 5));
    }
}

__kernel void simulate_poker_hands(
    __global const uchar* all_hands,
    __global int* histograms,
    const unsigned int num_hands,
    const unsigned int trials_per_hand,
    const unsigned int cards_per_hand,
    unsigned int seed
) {
    int hand_id = get_global_id(0);
    if (hand_id >= num_hands) return;

    // Local deck for each thread
    uchar deck[52];
    uchar community_cards[5]; // Always up to 5 community cards
    uchar full_hand[7];
    int histogram_offset = hand_id * NUM_BINS;

    // Caching or something weird can make it so these values are prefilled, so we need to reset them to 0
    for (int i = 0; i < NUM_BINS; i++) {
        histograms[histogram_offset + i] = 0;
    }

    __global const uchar* hand_cards = &all_hands[hand_id * cards_per_hand];

    // Calculate the number of known community cards
    int known_community_cards_amount = cards_per_hand - 2;

    for (unsigned int trial = 0; trial < trials_per_hand; trial++) {
        initialize_deck(deck);

        remove_hand_cards(deck, hand_cards, cards_per_hand); // Remove known hand cards

        // Reset community cards array
        for (int i = 0; i < 5; i++) community_cards[i] = 255; // Indicate empty
        // Copy known community cards from the hand
        for (int i = 0; i < known_community_cards_amount && i < 5; i++) {
            community_cards[i] = hand_cards[2 + i];
        }

        unsigned int current_seed = seed + trial + hand_id;
        draw_community_cards(deck, community_cards, known_community_cards_amount, &current_seed);


        // Manually copy cards to handle private to private copying
        copy_global_to_private(full_hand, hand_cards, 2); // Player's hole cards
        copy_private_to_private(full_hand + 2, community_cards, 5); // Community cards

        int player_score = evaluate_hand(full_hand, 7); // Evaluate player's full hand

        int opponents_beaten = 0;
        int total_opponent_hands = 0;

        for (int i = 0; i < 52; i++) {
            if (deck[i] == 255) continue;  // Skip used cards
            for (int j = i + 1; j < 52; j++) {  // Start from i + 1 to avoid duplicates
                if (deck[j] == 255) continue;  // Skip used cards
                uchar opponent_cards[2] = {deck[i], deck[j]};
                uchar opponent_full_hand[7];
                copy_private_to_private(opponent_full_hand, opponent_cards, 2);
                copy_private_to_private(opponent_full_hand + 2, community_cards, 5);
                int opponent_score = evaluate_hand(opponent_full_hand, 7);

                if (player_score > opponent_score) {
                    opponents_beaten += 2;
                } else if (player_score == opponent_score) {
                    opponents_beaten += 1;
                }
                total_opponent_hands += 2;
            }

        }

        float hand_strength = (float)opponents_beaten / (float)total_opponent_hands;
        int bin_index = (int)(hand_strength * (NUM_BINS - 1));
        // printf("Increasing in %d by 1\n", histogram_offset + bin_index);
        // printf("histogram value: %d", histograms[histogram_offset + bin_index]);
        atomic_inc(&histograms[histogram_offset + bin_index]);
    }
}
