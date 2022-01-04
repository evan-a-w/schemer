(define (even? x) 
        (= (modulo x 2) 0))

(define (vector->list vec)
        (foldr cons '() vec))

(define (list->vector list)
        (foldl vector-append! #() list))

(define (do func init lim)
        (let ((go (lambda (var)
                          (if (= var lim)
                              (func var)
                              (begin
                                (func var)
                                (go (+ var 1)))))))
        (go init)))

(defmacro (while WHILE_CONDI WHILE_EXPR) 
     (let ((WHILE_GO (open-lambda ()
          (if WHILE_CONDI
              (begin
                  WHILE_EXPR
                  (WHILE_GO))
              '()))))
          (WHILE_GO)))

(defmacro (var VAR_ID VAR_VAL VAR_REST)
          (let-deref ((VAR_ID VAR_VAL))
                     VAR_REST))

(defmacro (for FOR_ID in FOR_L FOR_EXPR)
          (let-deref ((FOR_ID '()))
                     (var FOR_GO (open-lambda (FOR__ACC, FOR__CURR)
                                              (begin (set-deref! FOR_ID FOR__CURR)
                                                     FOR_EXPR))
                     (foldl FOR_GO '() FOR_L))))
