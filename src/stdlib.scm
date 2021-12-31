(define (even? x) 
        (= (modulo x 2) 0))

(define (vector->list vec)
        (foldr cons '() vec))

(define (list->vector list)
        (foldl vector-append! #() list))
