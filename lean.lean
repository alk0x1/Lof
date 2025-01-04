def RangeProof (n : Nat) (max : Nat) : Prop :=
  n < max

theorem example_range : RangeProof 5 10 := by
  -- Prove 5 < 10
  simp [RangeProof]

def AddProof (a : Nat) (b : Nat) : Nat :=
  a + b

theorem example_add : AddProof 2 3 = 5 := by
  simp [AddProof]

def MultiplyProof (a : Nat) (b : Nat) : Nat :=
  a * b

theorem example_multiply : MultiplyProof 2 3 = 6 := by
  simp [MultiplyProof]
