(define sum 0)
(define (inc-apply curr)
        (if (>= curr 5)
            sum
            (begin
                (if (or (= (modulo curr 3) 0)
                        (= (modulo curr 5) 0))
                    (set! sum (+ sum curr))
                    '())
                (inc-apply (+ curr 1)))
        ))
(inc-apply 0)
