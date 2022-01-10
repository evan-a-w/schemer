(define (prime? x)
        (let ((lim (sqrt x))
              (go (lambda (curr)
                  (if (> curr lim)
                      #t
                      (if (= 0 (modulo x curr))
                          #f
                          (go (+ curr 1)))))))
             (go 2)))

(define val 600851475143)
(define (iter curr)
        (begin
            (display curr)
            (if (and (prime? curr) (= 0 (modulo val curr)))
                (begin (set! val (/ val curr)) curr)
                (+ curr 1))))
(apply-while (lambda (v) (< v val)) iter 2)
val
