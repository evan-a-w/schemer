(defmacro (name arg1)
    (eval.code<->data.eval (car (cdr ($FLIP arg1)))))

(defmacro (b arg1)
    (display ($FLIP arg1)))

(name ((display 1) (display 2) (display 3)))

(let ((x 0))
     (for i in '(1 2 3)
          (set! x (+ x i)))
     x)
